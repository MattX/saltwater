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

use saltwater_parser::get_str;
use saltwater_parser::InternedStr;
use serde::de::Visitor;
use serde::export::Formatter;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use lexpr::{Value, Number};
use std::convert::TryFrom;
use serde_lexpr::{from_str, to_string};
use itertools::Itertools;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MirExpr {
    Let(Box<Let>),
    Lambda(Box<Lambda>),
    If(Box<If>),
    Apply(Box<Apply>),
    Primitive(Primitive),
    Literal(Box<MirLiteral>),
    Ref(MirInternedStr),
    Comment(String, Box<MirExpr>),
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

    pub fn lambda(arg: MirInternedStr, body: MirExpr) -> MirExpr {
        MirExpr::Lambda(Box::new(Lambda { arg, body }))
    }

    pub fn let_(ident: MirInternedStr, value: MirExpr, body: MirExpr) -> MirExpr {
        MirExpr::Let(Box::new(Let { ident, value, body }))
    }

    pub fn nop() -> MirExpr {
        MirExpr::apply(
            MirExpr::Primitive(Primitive::Pure),
            MirExpr::literal(MirLiteral::Null),
        )
    }

    /// Desugar MIR
    ///  - Let into lambda
    ///  - High-level primitives into low-level primitives
    pub fn desugar(&self) -> MirExpr {
        match self {
            MirExpr::Let(let_) => {
                let Let { ident, value, body } = &**let_;
                MirExpr::apply(
                    MirExpr::lambda(ident.clone(), body.desugar()),
                    value.desugar(),
                )
            }
            _ => self.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Primitive {
    Plus,
    Minus,
    Times,
    Div,
    Mod,
    Neg,
    And,
    Or,
    Xor,
    Cons,
    Car,
    Cdr,
    Eq,
    Lt,
    Le,
    Gt,
    Ge,
    BoolToInt,

    // Higher level primitives -- get rewritten during desugaring
    Get(usize),
    Set(usize),
    Pure, // x -> S[x]
    Lift, // (x -> y) -> S[x] -> S[y]
    Then,
    Y,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MirLiteral {
    Bool(bool),
    Int(i64),
    Null,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Let {
    pub ident: MirInternedStr,
    pub value: MirExpr,
    pub body: MirExpr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Lambda {
    pub arg: MirInternedStr,
    pub body: MirExpr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct If {
    pub condition: MirExpr,
    pub consequent: MirExpr,
    pub alternative: MirExpr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Apply {
    pub func: MirExpr,
    pub arg: MirExpr,
}

/// This structure exists solely to implement Serialize and Deserialize
/// on [`InternedStr`](saltwater_parser::InternedStr).
///
/// Hopefully one day we get https://github.com/rust-lang/rfcs/pull/2393 and this
/// can go away.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Default)]
pub struct MirInternedStr(pub InternedStr);

impl MirInternedStr {
    pub fn is_empty(self) -> bool {
        self.0.is_empty()
    }
    pub fn resolve_and_clone(self) -> String {
        self.0.resolve_and_clone()
    }
    pub fn get_or_intern<T: AsRef<str> + Into<String>>(val: T) -> MirInternedStr {
        MirInternedStr(InternedStr::get_or_intern(val))
    }
}

impl std::fmt::Display for MirInternedStr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for MirInternedStr {
    fn from(s: &str) -> Self {
        Self::get_or_intern(s)
    }
}

impl From<String> for MirInternedStr {
    fn from(s: String) -> Self {
        Self::get_or_intern(s)
    }
}

impl From<InternedStr> for MirInternedStr {
    fn from(s: InternedStr) -> Self {
        Self(s)
    }
}

impl Serialize for MirInternedStr {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(get_str!(self.0))
    }
}

impl<'de> Deserialize<'de> for MirInternedStr {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        struct MirInternedStrVisitor;

        impl<'v> serde::de::Visitor<'v> for MirInternedStrVisitor {
            type Value = MirInternedStr;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("a string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(MirInternedStr::get_or_intern(v))
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(MirInternedStr::get_or_intern(v))
            }
        }

        deserializer.deserialize_str(MirInternedStrVisitor)
    }
}

pub fn lexpr_to_mir(v: lexpr::Value) -> Result<MirExpr, String> {
    Ok(match v {
        Value::Null => MirExpr::literal(MirLiteral::Null),
        Value::Bool(b) => MirExpr::literal(MirLiteral::Bool(b)),
        Value::Number(n) => if let Some(i) = n.as_i64() {
            MirExpr::literal(MirLiteral::Int(i))
        } else {
            return Err(format!("number not supported: {}", n));
        }
        Value::Symbol(s) => {
            match from_str::<Primitive>(&s) {
                Ok(p) => MirExpr::Primitive(p),
                Err(_) => MirExpr::Ref(MirInternedStr::get_or_intern(s)),
            }
        },
        Value::Cons(c) => cons_to_mir(c)?,
        Value::Keyword(k) => return Err(format!("keyword at toplevel: {:?}", k)),
        Value::Vector(_) | Value::Bytes(_)
        | Value::Char(_)
        | Value::Nil | Value::String(_)
        => return Err(format!("found unsupported object {:?}", v))
    })
}

fn cons_to_mir(cons: lexpr::Cons) -> Result<MirExpr, String> {
    let mut elems = into_vec_proper(cons)?;
    let first = elems.remove(0);
    match first {
        Value::Keyword(s) => parse_kw(&s, elems),
        _ => {
            let first_val = lexpr_to_mir(first)?;
            let mut others = elems.into_iter().map(lexpr_to_mir);
            others.fold_results(first_val, |func, arg| MirExpr::apply(func, arg))
        }
    }
}

fn into_vec_proper(cons: lexpr::Cons) -> Result<Vec<Value>, String> {
    let (vec, rest) = cons.into_vec();
    if !rest.is_null() {
        Err(format!("improper list: ends with {:?}", rest))
    } else {
        Ok(vec)
    }
}

fn parse_kw(kw: &str, mut elems: Vec<lexpr::Value>) -> Result<MirExpr, String> {
    // TODO refactor a bit to avoid this much repetition
    Ok(match kw {
        "let" => {
            if elems.len() != 2 {
                return Err(format!("let must have exactly two arguments, found {:?}", elems));
            }
            let body = lexpr_to_mir(elems.pop().unwrap())?;
            let mut bind = match elems.pop().unwrap() {
                Value::Cons(s) => into_vec_proper(s)?,
                e => return Err(format!("let first argument must be a pair, not {:?}", e)),
            };
            if bind.len() != 2 {
                return Err(format!("let binding must have exactly two elements, found {:?}", bind));
            }
            let val = lexpr_to_mir(bind.pop().unwrap())?;
            let ident = match bind.pop().unwrap() {
                Value::Symbol(s) => MirInternedStr::get_or_intern(s),
                e => return Err(format!("lambda first argument must be a symbol, not {:?}", e)),
            };
            MirExpr::let_(ident, val, body)
        }
        "lambda" => {
            if elems.len() != 2 {
                return Err(format!("lambda must have exactly two arguments, found {:?}", elems));
            }
            let body = lexpr_to_mir(elems.pop().unwrap())?;
            let ident = match elems.pop().unwrap() {
                Value::Symbol(s) => MirInternedStr::get_or_intern(s),
                e => return Err(format!("lambda first argument must be a symbol, not {:?}", e)),
            };
            MirExpr::lambda(ident, body)
        }
        "if" => {
            if elems.len() != 3 {
                return Err(format!("if must have exactly three arguments, found {:?}", elems));
            }
            let alternate = lexpr_to_mir(elems.pop().unwrap())?;
            let consequent = lexpr_to_mir(elems.pop().unwrap())?;
            let condition = lexpr_to_mir(elems.pop().unwrap())?;
            MirExpr::if_(condition, consequent, alternate)
        }
        "comment" => {
            if elems.len() != 2 {
                return Err(format!("comment must have exactly one argument, found {:?}", elems));
            }
            let body = lexpr_to_mir(elems.pop().unwrap())?;
            let comment = match elems.pop().unwrap() {
                Value::String(s) => s.to_string(),
                e => return Err(format!("comment first argument must be a string, not {:?}", e)),
            };
            MirExpr::Comment(comment, Box::new(body))
        }
        _ => return Err(format!("unknown keyword: {}", kw)),
    })
}

pub fn mir_to_lexpr(expr: &MirExpr) -> lexpr::Value {
    match expr {
        MirExpr::Let(l) => {
            Value::list(vec![
                Value::keyword("let".to_string()),
                Value::list(vec![Value::symbol(l.ident.to_string()), mir_to_lexpr(&l.value)]),
                mir_to_lexpr(&l.body),
            ])
        }
        MirExpr::Lambda(l) => {
            Value::list(vec![
                Value::keyword("lambda".to_string()),
                Value::symbol(l.arg.to_string()),
                mir_to_lexpr(&l.body),
            ])
        }
        MirExpr::If(if_) => {
            Value::list(vec![
                Value::keyword("if".to_string()),
                mir_to_lexpr(&if_.condition),
                mir_to_lexpr(&if_.consequent),
                mir_to_lexpr(&if_.alternative),
            ])
        }
        MirExpr::Apply(ap) => {
            Value::list(vec![
                mir_to_lexpr(&ap.func),
                mir_to_lexpr(&ap.arg),
            ])
        }
        MirExpr::Primitive(p) => Value::symbol(to_string(p).unwrap()),
        MirExpr::Literal(b) => match &**b {
            MirLiteral::Null => Value::Null,
            MirLiteral::Int(i) => Value::Number(Number::from(*i)),
            MirLiteral::Bool(b) => Value::Bool(*b),
        }
        MirExpr::Ref(r) => Value::symbol(r.to_string()),
        MirExpr::Comment(comment, body) => Value::list(vec![
            Value::string(comment.clone()),
            mir_to_lexpr(body),
        ])
    }
}

