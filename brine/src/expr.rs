// Copyright 2020 Matthieu Felix
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

use crate::mir::{MirExpr, MirLiteral, Primitive};
use crate::Compiler;
use saltwater_parser::hir::{Expr, ExprType};
use saltwater_parser::{CompileResult, LiteralValue, Location, Type};

pub struct Value {
    pub val: MirExpr,
    pub ctype: Type,
    pub pure: bool,
}

impl Compiler {
    pub fn compile_expr(&mut self, expr: Expr) -> CompileResult<Value> {
        todo!()
        /*
        let expr = expr.const_fold()?;
        match expr.expr {
            ExprType::Literal(token) => self.compile_literal(expr.ctype, token),
            ExprType::Id(var) => {
                let md = var.get();
                Ok(Value {
                    val: MirExpr::Primitive(Primitive::Get(*self.stack_positions.get(&md.id.into()).unwrap())),
                    ctype: md.ctype.clone(),
                    pure: false,
                })
            }
            _ => todo!("expression type not yet supported: {:?}", expr.expr),
        }
        */
    }

    fn compile_literal(&mut self, ctype: Type, token: LiteralValue) -> CompileResult<Value> {
        let val = match (token, &ctype) {
            (LiteralValue::Int(i), Type::Bool) => MirExpr::literal(MirLiteral::Bool(i != 0)),
            (LiteralValue::Int(i), _) => MirExpr::literal(MirLiteral::Int(i)),
            (LiteralValue::Char(i), _) => MirExpr::literal(MirLiteral::Int(i64::from(i))),
            _ => unimplemented!("only ints and bools are supported"),
        };
        Ok(Value {
            val,
            ctype,
            pure: true,
        })
    }
}
