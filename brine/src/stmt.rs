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
use crate::cfg::Jump;
use crate::create_res_lambda;
use crate::expr::Value;
use crate::mir::{MirExpr, MirLiteral, Primitive};
use crate::Compiler;
use saltwater_parser::data::hir::StmtType;
use saltwater_parser::hir::{Expr, Stmt};
use saltwater_parser::CompileResult;

impl Compiler {
    pub fn compile_all(&mut self, prev: Value, stmts: Vec<Stmt>) -> CompileResult<Value> {
        let mut v = prev;
        for stmt in stmts {
            v = self.compile_stmt(v, stmt)?;
        }
        Ok(todo!())
    }

    pub fn compile_stmt(&mut self, prev: Value, stmt: Stmt) -> CompileResult<Value> {
        match stmt.data {
            StmtType::Compound(stmts) => self.compile_all(prev, stmts),
            StmtType::Decl(decls) => {
                for decl in decls {
                    self.declare_stack(decl.data, decl.location);
                }
                Ok(prev)
            }
            StmtType::Return(expr) => {
                let retval = if let Some(e) = expr {
                    self.compile_expr(e)?.val
                } else {
                    MirExpr::literal(MirLiteral::Null)
                };
                self.cfg.add_instr(create_res_lambda(retval));
                self.cfg.set_jump(Jump::Jmp(self.return_block));
                Ok(prev)
            }
            StmtType::Expr(expr) => self.compile_expr(expr),
            //StmtType::If(condition, body, otherwise) => self.if_stmt(condition, *body, otherwise),
            _ => todo!("statement type not yet supported: {:?}", stmt.data),
        }
    }

    /*
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
    */
}
