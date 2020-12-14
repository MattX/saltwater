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
use crate::mir::{MirExpr, MirLiteral, Primitive};
use crate::{RETURN_CONT, create_res_lambda};
use crate::{Compiler, MirResult};
use saltwater_parser::data::hir::StmtType;
use saltwater_parser::hir::{Expr, Stmt};
use saltwater_parser::CompileResult;
use crate::cfg::{Jump, DoLine};

impl Compiler {
    pub fn compile_all(&mut self, stmts: Vec<Stmt>) -> CompileResult<()> {
        for stmt in stmts {
            if let Some(e) = self.compile_stmt(stmt)? {
                self.cfg.add_instr(create_res_lambda(e));
            }
        }
        Ok(())
    }

    pub fn compile_stmt(&mut self, stmt: Stmt) -> CompileResult<Option<MirExpr>> {
        match stmt.data {
            StmtType::Compound(stmts) => self.compile_all(stmts).map(|_| None),
            StmtType::Decl(decls) => {
                for decl in decls {
                    self.declare_stack(decl.data, decl.location)
                }
                Ok(None)
            },
            StmtType::Return(expr) => {
                let retval = if let Some(e) = expr {
                    self.compile_expr(e)?.val
                } else {
                    MirExpr::literal(MirLiteral::Null)
                };
                self.cfg.add_instr(create_res_lambda(retval));
                self.cfg.set_jump(Jump::Jmp(self.return_block));
                Ok(None)
            }
            StmtType::Expr(expr) => self.compile_expr(expr)?,
            StmtType::If(condition, body, otherwise) => self.if_stmt(condition, *body, otherwise),
            _ => todo!("statement type not yet supported: {:?}", stmt.data),
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
        let consequent = self.compile_stmt(consequent)?;
        let alternative = if let Some(alt) = alternative {
            self.compile_stmt(*alt)?
        } else {
            MirExpr::nop()
        };
        Ok(MirExpr::if_(condition, consequent, alternative))
    }
}
