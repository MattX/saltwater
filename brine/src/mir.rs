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
    LetCC(Box<LetCC>),
    Let(Box<Let>),
    Lambda(Box<Lambda>),
    If(Box<If>),
    Apply(Box<Apply>),
    PurePrimitive(Box<PurePrim>),
    StatePrimitive(Box<StatePrim>),
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

    pub fn let_cc(ident: InternedStr, body: MirExpr) -> MirExpr {
        MirExpr::LetCC(Box::new(LetCC { ident, body }))
    }

    pub fn state_primitive(sp: StatePrim) -> MirExpr {
        MirExpr::StatePrimitive(Box::new(sp))
    }

    pub fn if_(condition: MirExpr, consequent: MirExpr, alternative: MirExpr) -> MirExpr {
        MirExpr::If(Box::new(If {
            condition,
            consequent,
            alternative,
        }))
    }

    pub fn nop() -> MirExpr {
        MirExpr::apply(
            MirExpr::state_primitive(StatePrim::Pure),
            MirExpr::literal(MirLiteral::Null),
        )
    }
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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
    pub bindings: Vec<(InternedStr, MirExpr)>,
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
