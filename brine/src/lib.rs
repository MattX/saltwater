#[macro_use]
extern crate lazy_static;

mod ast;
mod cfg;
mod expr;
mod mir;
mod miri;
mod stmt;

use crate::ast::SyntaxNode;
use crate::mir::{MirExpr, MirInternedStr};
use saltwater_parser::get_str;
use saltwater_parser::hir::{Declaration, Initializer, Stmt, Symbol};
use saltwater_parser::types::FunctionType;
use saltwater_parser::{
    CompileError, CompileResult, ErrorHandler, InternedStr, Locatable, Location, Opt, Program,
    StorageClass, Type,
};
use std::collections::HashMap;
use std::convert::TryFrom;

/// Compile and return the declarations and warnings.
pub fn compile(buf: &str, opt: Opt) -> Program<MirExpr> {
    use saltwater_parser::{check_semantics, vec_deque};

    let program = check_semantics(buf, opt);
    let hir = match program.result {
        Ok(hir) => hir,
        Err(err) => {
            return Program {
                result: Err(err),
                warnings: program.warnings,
                files: program.files,
            }
        }
    };
    // really we'd like to have all errors but that requires a refactor
    let mut err = None;
    let mut compiler = Compiler::new();
    let mut func_code = HashMap::<InternedStr, MirExpr>::new();
    for decl in hir {
        let meta = decl.data.symbol.get();
        if let StorageClass::Typedef = meta.storage_class {
            continue;
        }
        let current = match &meta.ctype {
            Type::Function(func_type) => match decl.data.init {
                Some(Initializer::FunctionBody(stmts)) => {
                    match compiler.compile_func(decl.data.symbol, &func_type, stmts, decl.location)
                    {
                        Ok(expr) => {
                            func_code.insert(decl.data.symbol.get().id, expr);
                            Ok(())
                        }
                        Err(e) => Err(e),
                    }
                }
                None => Ok(()),
                _ => unreachable!("functions can only be initialized by a FunctionBody"),
            },
            Type::Void | Type::Error => unreachable!("parser let an incomplete type through"),
            _ => {
                if let Some(Initializer::FunctionBody(_)) = &decl.data.init {
                    unreachable!("only functions should have a function body")
                }
                todo!("Store static")
            }
        };
        if let Err(e) = current {
            err = Some(e);
            break;
        }
    }
    let result = if let Some(err) = err {
        Err(err)
    } else {
        Ok(func_code
            .remove(&InternedStr::get_or_intern("main"))
            .unwrap())
    };
    Program {
        result: result.map_err(|errs| vec_deque![errs]),
        warnings: program.warnings,
        files: program.files,
    }
}

#[derive(Default)]
struct Compiler {
    pub locals: Vec<Vec<InternedStr>>,
    pub gensym_counter: usize,
}

pub type MirResult = CompileResult<MirExpr>;

lazy_static! {
    pub static ref RETURN_CONT: MirInternedStr = MirInternedStr::get_or_intern("__return_cont");
}

impl Compiler {
    pub fn new() -> Compiler {
        Compiler::default()
    }

    pub fn gensym(&mut self, prefix: &str) -> MirInternedStr {
        let i = self.gensym_counter;
        self.gensym_counter += 1;
        MirInternedStr::get_or_intern(format!("__{}_{}", prefix, i))
    }

    fn compile_func(
        &mut self,
        symbol: Symbol,
        func_type: &FunctionType,
        stmts: Vec<Stmt>,
        location: Location,
    ) -> MirResult {
        self.compile_all(stmts);
        todo!()
    }
}
