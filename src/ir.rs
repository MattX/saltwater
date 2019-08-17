use std::convert::{TryFrom, TryInto};

use cranelift::codegen::{
    self,
    ir::{condcodes, function::Function, ExternalName, InstBuilder},
    isa,
    settings::{self, Configurable},
};
use cranelift::prelude::{
    types, FunctionBuilder, FunctionBuilderContext, Type as IrType, Value as IrValue,
};
use cranelift_faerie::{FaerieBackend, FaerieBuilder, FaerieTrapCollection};
use cranelift_module::{self, DataContext, Linkage, Module as CraneliftModule};

use crate::backend::TARGET;
use crate::data::{
    ArrayType, Declaration, Expr, ExprType, FunctionType, Initializer, Locatable, Location,
    Qualifiers, Stmt, StorageClass, Symbol, Token, Type,
};
use crate::utils::warn;

type Module = CraneliftModule<FaerieBackend>;
type IrResult = Result<Value, Locatable<String>>;

struct LLVMCompiler {
    // TODO: allow compiling multiple modules with the same compiler struct?
    module: Module,
}

struct Value {
    ir_val: IrValue,
    ir_type: IrType,
    ctype: Type,
}

/// Compile a program from a high level IR to a Cranelift Module
pub(crate) fn compile(program: Vec<Locatable<Declaration>>) -> Result<Module, Locatable<String>> {
    let name = program
        .first()
        .map_or_else(|| "<empty>".to_string(), |decl| decl.data.symbol.id.clone());
    let mut compiler = LLVMCompiler::new(name);
    for decl in program {
        match (decl.data.symbol.ctype.clone(), decl.data.init) {
            (_, None) => {} // only compile definitions
            (Type::Void, _) => panic!("parser let an incomplete type through"),
            (Type::Function(func_type), Some(Initializer::FunctionBody(stmts))) => compiler
                .compile_func(
                    decl.data.symbol.id,
                    func_type,
                    decl.data.symbol.qualifiers,
                    decl.data.symbol.storage_class,
                    stmts,
                    decl.location,
                )?,
            (Type::Function(_), _) => panic!("functions should have a FunctionBody initializer"),
            (_, Some(Initializer::FunctionBody(_))) => {
                panic!("only functions should have a function body")
            }
            (_, init) => compiler.store_static(decl.data.symbol, init, decl.location)?,
        }
    }
    Ok(compiler.module)
}

macro_rules! scalar_bin_op {
        ($self: expr, $left: expr, $right: expr, $builder: expr, $($pat: pat $(if $guard: expr)?, $func: ident),+) => {{
            let (left, right) = ($self.rval($left, $builder)?, $self.rval($right, $builder)?);
            assert_eq!(left.ir_type, right.ir_type);
            match (left.ir_type, left.ctype.is_signed()) {
                $($pat $(if $guard)? => Ok(Value {
                    ir_val: $ builder.ins().$ func(left.ir_val, right.ir_val),
                    ir_type: left.ir_type,
                    // TODO: this will probably be wrong for pointer addition
                    ctype: left.ctype,
                }) ),+ ,
                _ => unreachable!("parser should have caught illegal IR types")
            }
        }};
    }

impl LLVMCompiler {
    fn new(name: String) -> LLVMCompiler {
        let mut flags_builder = settings::builder();
        // allow creating shared libraries
        flags_builder
            .enable("is_pic")
            .expect("is_pic should be a valid option");
        // use debug assertions
        flags_builder
            .enable("enable_verifier")
            .expect("enable_verifier should be a valid option");
        // compile quickly, but without optimizations
        flags_builder
            .set("opt_level", "fastest")
            .expect("opt_level: fastest should be a valid option");

        let isa = isa::lookup(TARGET.clone())
            .unwrap_or_else(|_| panic!("platform not supported: {}", *TARGET))
            .finish(settings::Flags::new(flags_builder));

        let builder = FaerieBuilder::new(
            isa,
            name,
            FaerieTrapCollection::Disabled,
            cranelift_module::default_libcall_names(),
        )
        .expect("unknown error creating module");

        LLVMCompiler {
            module: Module::new(builder),
        }
    }
    fn compile_func(
        &mut self,
        id: String,
        func_type: FunctionType,
        quals: Qualifiers,
        sc: StorageClass,
        stmts: Vec<Stmt>,
        location: Location,
    ) -> Result<(), Locatable<String>> {
        let signature = func_type.signature(&location)?;
        let linkage = sc.try_into().map_err(|err| Locatable {
            data: err,
            location,
        })?;
        let func_id = self
            .module
            .declare_function(&id, linkage, &signature)
            .expect("should not have an error declaring a function");
        // external name is meant to be a lookup in a symbol table,
        // but we just give it garbage values
        let mut func = Function::with_name_signature(ExternalName::user(0, 0), signature);

        // this context is just boiler plate
        let mut ctx = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut func, &mut ctx);

        let func_start = builder.create_ebb();
        builder.append_ebb_params_for_function_params(func_start);
        builder.switch_to_block(func_start);

        self.compile_all(stmts, &mut builder)?;

        let flags = settings::Flags::new(settings::builder());
        codegen::verify_function(&func, &flags).expect("should not have a compile error");

        let mut ctx = codegen::Context::for_function(func);
        self.module
            .define_function(func_id, &mut ctx)
            .expect("should not have an error defining a function");
        Ok(())
    }
    fn compile_all(
        &mut self,
        stmts: Vec<Stmt>,
        builder: &mut FunctionBuilder,
    ) -> Result<(), Locatable<String>> {
        for stmt in stmts {
            self.compile_stmt(stmt, builder)?;
        }
        Ok(())
    }
    fn compile_stmt(
        &mut self,
        stmt: Stmt,
        builder: &mut FunctionBuilder,
    ) -> Result<(), Locatable<String>> {
        match stmt {
            Stmt::Compound(stmts) => self.compile_all(stmts, builder)?,
            Stmt::Return(expr) => {
                let mut ret = vec![];
                if let Some(e) = expr {
                    let val = self.compile_expr(e, builder)?;
                    ret.push(val.ir_val);
                }
                builder.ins().return_(&ret);
            }
            Stmt::If(condition, body, otherwise) => {
                // If condtion is zero:
                //      If else_block exists, jump to else_block + compile_all
                //      Otherwise, jump to end_block
                //  Otherwise:
                //      Fallthrough to if_body + compile_all
                //      If else_block exists, jump to end_block + compile_all
                //      Otherwise, fallthrough to end_block
                /*
                let condition = self.compile_expr(condition);
                let (if_body, end_body) = (builder.create_ebb(), builder.create_ebb());
                let target = if let Some(o) = otherwise {
                    let else_body = builder.create_ebb();
                    builder.switch_to_block(else_body);
                    self.compile_all(o, builder);
                    builder.switch_to_block(
                    else_body
                } else {
                    end_body
                };
                builder.ins().brz(condtion, target, &[]);
                */
                unimplemented!("if statements");
                //let reg = self.compile_expr(condition);
            }
            _ => unimplemented!("almost every statement"),
        };
        Ok(())
    }
    fn compile_expr(&self, expr: Expr, builder: &mut FunctionBuilder) -> IrResult {
        let location = expr.location;
        let ir_type = match expr.ctype.as_ir_basic_type() {
            Ok(ir_type) => ir_type,
            Err(err) => {
                return Err(Locatable {
                    data: err,
                    location,
                });
            }
        };
        match expr.expr {
            ExprType::Literal(token) => self.compile_literal(ir_type, expr.ctype, token, builder),
            ExprType::Cast(orig, ctype) => self.cast(*orig, ctype, location, builder),
            ExprType::Id(var) => self.load_addr(var),
            ExprType::Add(left, right) => self.add(*left, *right, builder),
            ExprType::Sub(left, right) => scalar_bin_op!(self, *left, *right, builder,
                (ty, _) if ty.is_int(), isub,
                (ty, _) if ty.is_float(), fsub),
            ExprType::Mul(left, right) => scalar_bin_op!(self, *left, *right, builder,
                (ty, _) if ty.is_int(), imul,
                (ty, _) if ty.is_float(), fmul),
            ExprType::Div(left, right) => scalar_bin_op!(self, *left, *right, builder,
                (ty, true) if ty.is_int(), sdiv,
                (ty, false) if ty.is_int(), udiv,
                (ty, _) if ty.is_float(), fdiv),
            ExprType::BitwiseAnd(left, right) => scalar_bin_op!(self, *left, *right, builder,
               (ty, true) if ty.is_int(), band),
            ExprType::BitwiseOr(left, right) => scalar_bin_op!(self, *left, *right, builder,
               (ty, true) if ty.is_int(), bor),
            // left shift
            ExprType::Shift(left, right, true) => scalar_bin_op!(self, *left, *right, builder,
               (ty, _) if ty.is_int(), ishl),
            // right shift
            ExprType::Shift(left, right, false) => scalar_bin_op!(self, *left, *right, builder,
                // arithmetic shift: keeps the sign of `left`
               (ty, true) if ty.is_int(), sshr,
               // logical shift: shifts in zeros
               (ty, false) if ty.is_int(), ushr),
            ExprType::Xor(left, right) => scalar_bin_op!(self, *left, *right, builder,
                                                         (ty, _) if ty.is_int(), bxor),
            ExprType::Compare(left, right, token) => self.compare(*left, *right, &token, builder),
            _ => {
                unimplemented!("unary expressions, assignments, ternary, comma, logical expressions, sizeof, (casts), struct operations")
            }
        }
    }
    fn compile_literal(
        &self,
        ir_type: IrType,
        ctype: Type,
        token: Token,
        builder: &mut FunctionBuilder,
    ) -> IrResult {
        match token {
            Token::Int(i) => Ok(Value {
                ir_val: builder.ins().iconst(ir_type, i),
                ir_type,
                ctype,
            }),
            Token::Float(f) => Ok(Value {
                ir_val: builder.ins().f64const(f),
                ir_type: types::F64,
                ctype,
            }),
            _ => unimplemented!("aggregate literals"),
        }
    }
    fn cast(
        &self,
        expr: Expr,
        ctype: Type,
        location: Location,
        builder: &mut FunctionBuilder,
    ) -> IrResult {
        // calculate this here before it's moved to `compile_expr`
        let orig_signed = expr.ctype.is_signed();
        let original = self.compile_expr(expr, builder)?;
        let cast_type = ctype.as_ir_basic_type().map_err(|err| Locatable {
            data: err,
            location,
        })?;
        // NOTE: we compare the IR types, not the C types, because multiple C types
        // NOTE: may have the same representation (e.g. both `int` and `long` are i64)
        if cast_type == original.ir_type {
            // no-op
            return Ok(original);
        }
        let cast = match (original.ir_type, cast_type) {
            (types::F32, types::F64) => builder.ins().fpromote(cast_type, original.ir_val),
            (types::F64, types::F32) => builder.ins().fdemote(cast_type, original.ir_val),
            (b, i) if b.is_bool() && i.is_int() => builder.ins().bint(cast_type, original.ir_val),
            (i, b) if i.is_int() && b.is_bool() => {
                builder
                    .ins()
                    .icmp_imm(condcodes::IntCC::NotEqual, original.ir_val, 0)
            }
            (i, f) if i.is_int() && f.is_float() => {
                if orig_signed {
                    builder.ins().fcvt_from_sint(cast_type, original.ir_val)
                } else {
                    builder.ins().fcvt_from_uint(cast_type, original.ir_val)
                }
            }
            (f, i) if f.is_float() && i.is_int() => {
                if orig_signed {
                    builder.ins().fcvt_to_sint(cast_type, original.ir_val)
                } else {
                    builder.ins().fcvt_to_uint(cast_type, original.ir_val)
                }
            }
            _ => unimplemented!("cast from {} to {}", original.ir_type, cast_type),
        };
        Ok(Value {
            ir_val: cast,
            ir_type: cast_type,
            ctype,
        })
    }
    fn load_addr(&self, var: Symbol) -> IrResult {
        unimplemented!("address loads");
    }
    fn add(&self, left: Expr, right: Expr, builder: &mut FunctionBuilder) -> IrResult {
        scalar_bin_op!(self, left, right, builder, (ty, _) if ty.is_int(), iadd, (ty, _) if ty.is_float(), fadd)
    }
    fn compare(
        &self,
        left: Expr,
        right: Expr,
        token: &Token,
        builder: &mut FunctionBuilder,
    ) -> IrResult {
        let (left, right) = (self.rval(left, builder)?, self.rval(right, builder)?);
        assert_eq!(left.ir_type, right.ir_type);

        let ir_val = if left.ir_type.is_int() {
            let code = token
                .to_int_compare(left.ctype.is_signed())
                .expect("Expr::Compare should only have comparison tokens");
            builder.ins().icmp(code, left.ir_val, right.ir_val)
        } else {
            assert!(left.ir_type.is_float());
            let code = token
                .to_float_compare()
                .expect("Expr::Compare should only have comparison tokens");
            builder.ins().fcmp(code, left.ir_val, right.ir_val)
        };
        Ok(Value {
            ir_val,
            ir_type: types::B1,
            ctype: left.ctype,
        })
    }
    fn rval(&self, expr: Expr, builder: &mut FunctionBuilder) -> IrResult {
        let compiled = self.compile_expr(expr, builder)?;
        // TODO: deref lvals
        // TODO: will structs need special handling?
        Ok(compiled)
    }
    fn store_static(
        &mut self,
        symbol: Symbol,
        init: Option<Initializer>,
        location: Location,
    ) -> Result<(), Locatable<String>> {
        let err_closure = |err| Locatable {
            data: err,
            location: location.clone(),
        };
        let linkage = symbol.storage_class.try_into().map_err(err_closure)?;
        let id = self
            .module
            .declare_data(
                &symbol.id,
                linkage,
                symbol.qualifiers.c_const,
                Some(
                    symbol
                        .ctype
                        .alignof()
                        .map_err(|err| err.to_string())
                        .and_then(|size| {
                            size.try_into().map_err(|f| {
                                format!("align of {} is greater than 256 bytes", symbol.id)
                            })
                        })
                        .map_err(err_closure)?,
                ),
            )
            .map_err(|err| Locatable {
                data: format!("error storing static value: {}", err),
                location: location.clone(),
            })?;
        let mut ctx = DataContext::new();
        if let Some(init) = init {
            let data = init.into_bytes(&symbol.ctype, &location)?;
            ctx.define(data)
        } else {
            ctx.define_zeroinit(
                symbol
                    .ctype
                    .sizeof()
                    .map_err(|err| err_closure(err.to_string()))? as usize,
            )
        };
        self.module.define_data(id, &ctx).map_err(|err| Locatable {
            data: format!("error defining static variable: {}", err),
            location,
        })
    }
}

impl Initializer {
    fn into_bytes(self, ctype: &Type, location: &Location) -> Result<Box<[u8]>, Locatable<String>> {
        match self {
            Initializer::InitializerList(mut initializers) => match ctype {
                Type::Array(ty, ArrayType::Unbounded) => {
                    unimplemented!("inferring size of unbounded array from initializer")
                }
                Type::Array(ty, ArrayType::Fixed(size)) => {
                    let size = *size;
                    if initializers.len() as u64 > size {
                        Err(Locatable {
                            data: format!(
                                "too many initalizers for array (expected {}, got {})",
                                size,
                                initializers.len()
                            ),
                            location: location.clone(),
                        })
                    } else {
                        let init_len = initializers.len();
                        let mut bytes = vec![];
                        let elements = initializers
                            .into_iter()
                            .map(|init| init.into_bytes(ctype, location));

                        for init in elements {
                            bytes.extend_from_slice(&*init?);
                        }
                        let sizeof = ty.sizeof().map_err(|err| Locatable {
                            location: location.clone(),
                            data: err.to_string(),
                        })?;
                        let remaining_bytes = sizeof * (size - init_len as u64);
                        let mut zero_init = Vec::new();
                        zero_init.resize(remaining_bytes as usize, 0);
                        bytes.append(&mut zero_init);

                        Ok(bytes.into_boxed_slice())
                    }
                }
                ty if ty.is_scalar() => {
                    assert_eq!(initializers.len(), 1);
                    initializers.remove(0).into_bytes(ctype, location)
                }
                Type::Union(_) => unimplemented!("union initializers"),
                Type::Struct(_) => unimplemented!("struct initializers"),
                Type::Bitfield(_) => unimplemented!("bitfield initalizers"),

                Type::Function(_) => unreachable!("function initializers"),
                Type::Void => unreachable!("initializer for void type"),
                _ => unreachable!("scalar types should have been handled"),
            },
            Initializer::Scalar(expr) => expr.into_bytes(),
            Initializer::FunctionBody(_) => {
                panic!("function definitions should go through compile_function, not store_static")
            }
        }
    }
}

impl Expr {
    fn into_bytes(self) -> Result<Box<[u8]>, Locatable<String>> {
        match self.const_fold() {
            Some(constexpr) => constexpr.data.into_bytes(self.ctype, &constexpr.location),
            None => Err(Locatable {
                data: "expression is not a compile time constant".into(),
                location: self.location,
            }),
        }
    }
    pub(crate) fn const_fold(&self) -> Option<Locatable<Token>> {
        if self.constexpr {
            let constexpr = match self.expr {
                ExprType::Literal(ref token) => token,
                _ => unimplemented!("const folding"),
            };
            Some(Locatable {
                location: self.location.clone(),
                data: constexpr.clone(),
            })
        } else {
            None
        }
    }
}

macro_rules! cast {
    ($i: expr, $from: ty, $to: ty, $ctype: expr, $location: expr) => {{
        let cast = $i as $to;
        if cast as $from != $i {
            warn(
                &format!(
                    "conversion to {} loses precision ({} != {})",
                    $ctype, cast as $from, $i
                ),
                $location,
            )
        }
        cast
    }};
}

macro_rules! bytes {
    ($int: expr, $be: expr) => {{
        let boxed: Box<[u8]> = if $be {
            Box::new($int.to_be_bytes())
        } else {
            Box::new($int.to_le_bytes())
        };
        boxed
    }};
}

impl Token {
    fn into_bytes(self, ctype: Type, location: &Location) -> Result<Box<[u8]>, Locatable<String>> {
        let ir_type = match ctype.clone().as_ir_basic_type() {
            Err(err) => {
                return Err(Locatable {
                    data: err,
                    location: location.clone(),
                })
            }
            Ok(ir_type) => ir_type,
        };
        //let mut buf = [u8; ir_type.bytes()];
        let big_endian = TARGET
            .endianness()
            .expect("target should be big or little endian")
            == target_lexicon::Endianness::Big;

        match self {
            Token::Int(i) => Ok(match ir_type {
                types::I8 => bytes!(cast!(i, i64, i8, &ctype, &location), big_endian),
                types::I16 => bytes!(cast!(i, i64, i16, &ctype, &location), big_endian),
                types::I32 => bytes!(cast!(i, i64, i32, &ctype, &location), big_endian),
                types::I64 => bytes!(i, big_endian),
                x => unreachable!(format!(
                    "ir_type {} for integer {} is not of integer type",
                    x, i
                )),
            }),
            Token::Float(f) => Ok(match ir_type {
                types::F32 => {
                    let cast = f as f32;
                    if (f64::from(cast) - f).abs() >= std::f64::EPSILON {
                        warn(&format!("conversion from double to float loses precision ({} is different from {} by more than DBL_EPSILON ({}))",
                        f64::from(cast), f, std::f64::EPSILON), &location);
                    }
                    let float_as_int = unsafe { *(&cast as *const f32 as *const u32) };
                    bytes!(float_as_int, big_endian)
                }
                types::F64 => {
                    let float_as_int = unsafe { *(&f as *const f64 as *const u64) };
                    bytes!(float_as_int, big_endian)
                }
                x => unreachable!(format!(
                    "ir_type {} for float {} is not of integer type",
                    x, f
                )),
            }),
            Token::Str(string) => Ok(string.into_boxed_str().into()),
            Token::Char(c) => Ok(Box::new([c])),
            x => unimplemented!("storing static of type {:?}", x),
        }
    }
}

impl TryFrom<StorageClass> for Linkage {
    type Error = String;
    fn try_from(sc: StorageClass) -> Result<Linkage, String> {
        match sc {
            StorageClass::Extern => Ok(Linkage::Export),
            StorageClass::Static => Ok(Linkage::Local),
            StorageClass::Auto | StorageClass::Register => {
                Err(format!("illegal storage class {} for global variable", sc))
            }
        }
    }
}