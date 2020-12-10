mod ast;
mod stmt;
mod expr;
mod mir;

use saltwater_parser::{Opt, Program, StorageClass, Type, InternedStr, ErrorHandler, CompileResult, Location, CompileError, Locatable};
use saltwater_parser::hir::{Initializer, Symbol, Declaration, Stmt};
use saltwater_parser::get_str;
use std::collections::HashMap;
use std::convert::TryFrom;
use saltwater_parser::types::FunctionType;
use crate::ast::SyntaxNode;
use crate::mir::MirExpr;

/// Compile and return the declarations and warnings.
pub fn compile(buf: &str, opt: Opt) -> Program<MirExpr> {
    use saltwater_parser::{check_semantics, vec_deque};

    let mut program = check_semantics(buf, opt);
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
                    match compiler.compile_func(decl.data.symbol, &func_type, stmts, decl.location) {
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
        Ok(func_code.remove(&InternedStr::get_or_intern("main")).unwrap())
    };
    Program {
        result: result.map_err(|errs| vec_deque![errs]),
        warnings: program.warnings,
        files: program.files,
    }
}

struct Compiler {}

pub type MirResult = CompileResult<MirExpr>;

impl Compiler {
    fn new() -> Compiler {
        Compiler {}
    }

    fn compile_func(
        &mut self,
        symbol: Symbol,
        func_type: &FunctionType,
        stmts: Vec<Stmt>,
        location: Location,
    ) -> MirResult {
        if stmts.len() != 1 {
            todo!("Support more than one statement")
        }
        self.compile_stmt(stmts.into_iter().next().unwrap())
    }
}
