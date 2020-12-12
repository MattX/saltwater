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

//! ## Mid-level intermediate representation
//! Describes a purely-functional language higher-level than Relambda, serving as an intermediate
//! compilation step.

use saltwater_parser::InternedStr;

#[derive(Debug, Clone)]
pub enum MirExpr {
    Let(Box<Let>),
    Lambda(Box<Lambda>),
    If(Box<If>),
    Apply(Box<Apply>),
    PurePrim(PurePrim),
    StatePrim(StatePrim),
    Literal(Box<MirLiteral>),
    Ref(InternedStr),
    Do(Vec<MirExpr>),
}

impl MirExpr {
    pub fn apply(func: MirExpr, arg: MirExpr) -> MirExpr {
        MirExpr::Apply(Box::new(Apply { func, arg }))
    }

    pub fn literal(ml: MirLiteral) -> MirExpr {
        MirExpr::Literal(Box::new(ml))
    }

    pub fn if_(condition: MirExpr, consequent: MirExpr, alternative: MirExpr) -> MirExpr {
        MirExpr::If(Box::new(If {
            condition,
            consequent,
            alternative,
        }))
    }

    pub fn lambda(arg: InternedStr, body: MirExpr) -> MirExpr {
        MirExpr::Lambda(Box::new(Lambda { arg, body }))
    }

    pub fn nop() -> MirExpr {
        MirExpr::apply(
            MirExpr::StatePrim(StatePrim::Pure),
            MirExpr::literal(MirLiteral::Null),
        )
    }

    /// Desugar MIR
    ///  - Do into sequenced then
    ///  - Let into lambda
    pub fn desugar(&self) -> MirExpr {
        match self {
            MirExpr::Let(let_) => {
                let Let { ident, value, body } = &**let_;
                MirExpr::apply(
                    MirExpr::lambda(ident.clone(), body.desugar()),
                    value.desugar(),
                )
            }
            MirExpr::Do(_) => todo!(),
            _ => self.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PurePrim {
    Plus,
    Minus,
    Times,
    Div,
    Mod,
    Neg,
    And,
    Or,
    Cons,
    Car,
    Cdr,
    IntToBool,
    BoolToInt,
}

#[derive(Debug, Clone, Copy)]
pub enum StatePrim {
    Get(InternedStr),
    Set(InternedStr),
    Pure,
}

#[derive(Debug, Clone)]
pub enum MirLiteral {
    Bool(bool),
    Int(i64),
    Null,
}

#[derive(Debug, Clone)]
pub struct Let {
    pub ident: InternedStr,
    pub value: MirExpr,
    pub body: MirExpr,
}

#[derive(Debug, Clone)]
pub struct LetCC {
    pub ident: InternedStr,
    pub body: MirExpr,
}

#[derive(Debug, Clone)]
pub struct Lambda {
    pub arg: InternedStr,
    pub body: MirExpr,
}

#[derive(Debug, Clone)]
pub struct If {
    pub condition: MirExpr,
    pub consequent: MirExpr,
    pub alternative: MirExpr,
}

#[derive(Debug, Clone)]
pub struct Apply {
    pub func: MirExpr,
    pub arg: MirExpr,
}
