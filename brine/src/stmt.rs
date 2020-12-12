// Copyright 2019 Matthieu Felix
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::ast::SyntaxNode;
use crate::mir::{MirExpr, MirLiteral, StatePrim};
use crate::RETURN_CONT;
use crate::{Compiler, MirResult};
use saltwater_parser::data::hir::StmtType;
use saltwater_parser::hir::{Expr, Stmt};
use saltwater_parser::CompileResult;

impl Compiler {
    pub fn compile_all(&mut self, stmts: Vec<Stmt>) -> MirResult {
        let mut exprs = Vec::new();
        let stmt_count = stmts.len();
        for (i_stmt, stmt) in stmts.into_iter().enumerate() {
            let last = i_stmt == stmt_count - 1;
            exprs.push(self.compile_stmt(stmt, last)?);
        }
        Ok(MirExpr::Do(exprs))
    }

    pub fn compile_stmt(&mut self, stmt: Stmt, last: bool) -> MirResult {
        // FIXME last should actually be a tail position check, this makes no sense.
        match stmt.data {
            StmtType::Compound(stmts) => self.compile_all(stmts),
            // FIXME just skip this instead?
            StmtType::Decl(_) => Ok(MirExpr::nop()),
            StmtType::Return(expr) => {
                let retval = if let Some(e) = expr {
                    self.compile_expr(e)?.val
                } else {
                    MirExpr::literal(MirLiteral::Null)
                };
                if last {
                    Ok(retval)
                } else {
                    Ok(MirExpr::apply(
                        MirExpr::StatePrim(StatePrim::Get(*RETURN_CONT)),
                        retval,
                    ))
                }
            }
            StmtType::Expr(expr) => Ok(self.compile_expr(expr)?.val),
            StmtType::If(condition, body, otherwise) => self.if_stmt(condition, *body, otherwise),
            _ => todo!(),
        }
    }

    fn if_stmt(
        &mut self,
        condition: Expr,
        consequent: Stmt,
        alternative: Option<Box<Stmt>>,
    ) -> MirResult {
        // TODO do I need to check the ctype here?
        let condition = self.compile_expr(condition)?.val;
        let consequent = self.compile_stmt(consequent, false)?;
        let alternative = if let Some(alt) = alternative {
            self.compile_stmt(*alt, false)?
        } else {
            MirExpr::nop()
        };
        Ok(MirExpr::if_(condition, consequent, alternative))
    }
}
