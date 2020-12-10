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

#[derive(Debug, Clone)]
pub enum MirExpr {
    Let(Vec<(String, MirExpr)>),
    Lambda(Box<Lambda>),
    If(Box<If>),
    Apply(Box<Apply>),
    PurePrimitive(Box<PurePrim>),
    StatePrimitive(Box<StatePrim>),
    Literal(Box<MirLiteral>),
}

impl MirExpr {
    pub fn literal(ml: MirLiteral) -> MirExpr {
        MirExpr::Literal(Box::new(ml))
    }
}

#[derive(Debug, Clone)]
pub enum PurePrim {
    Plus(MirExpr, MirExpr),
    Minus(MirExpr, MirExpr),
    Times(MirExpr, MirExpr),
    Div(MirExpr, MirExpr),
    Mod(MirExpr, MirExpr),
    Neg(MirExpr),
    And(MirExpr, MirExpr),
    Or(MirExpr, MirExpr),
    Cons(MirExpr, MirExpr),
    Car(MirExpr),
    Cdr(MirExpr),
    IntToBool(MirExpr),
    BoolToInt(MirExpr),
}

#[derive(Debug, Clone)]
pub enum StatePrim {
    Push(Vec<(String, MirExpr)>),
    Pop(Vec<String>),
    Get(String, MirExpr),
    Put(String, MirExpr),
}

#[derive(Debug, Clone)]
pub enum MirLiteral {
    Bool(bool),
    Int(i64),
    Null,
}

#[derive(Debug, Clone)]
pub struct Lambda {
    pub arg: String,
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
