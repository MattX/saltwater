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
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use serde::export::Formatter;
use serde::de::{Visitor};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MirExpr {
    Let(Box<Let>),
    Lambda(Box<Lambda>),
    If(Box<If>),
    Apply(Box<Apply>),
    PurePrim(PurePrim),
    StatePrim(StatePrim),
    Literal(Box<MirLiteral>),
    Ref(MirInternedStr),
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

    pub fn lambda(arg: MirInternedStr, body: MirExpr) -> MirExpr {
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StatePrim {
    Get(MirInternedStr),
    Set(MirInternedStr),
    Pure,
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
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error> where S: Serializer {
        serializer.serialize_str(get_str!(self.0))
    }
}

impl<'de> Deserialize<'de> for MirInternedStr {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error> where D: Deserializer<'de> {
        struct MirInternedStrVisitor;

        impl<'v> serde::de::Visitor<'v> for MirInternedStrVisitor {
            type Value = MirInternedStr;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("a string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> where E: serde::de::Error {
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
