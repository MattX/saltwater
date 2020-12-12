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

//! ## Miri -- an explicit-CPS interpreter for MIR

use crate::mir::{Apply, If, MirExpr, MirLiteral, PurePrim, StatePrim, MirInternedStr};
use saltwater_parser::InternedStr;
use std::rc::Rc;

/// A Mir runtime object
#[derive(Debug, Clone)]
pub enum MirObj<'a> {
    Bool(bool),
    Int(i64),
    Null,
    Lambda(Box<Lambda<'a>>),
    PurePrim(PurePrim),
    StatePrim(StatePrim),
    Cons(Rc<MirObj<'a>>, Rc<MirObj<'a>>),
}

#[derive(Debug, Clone)]
pub struct Lambda<'a> {
    env: RcEnv<'a>,
    arg: MirInternedStr,
    body: &'a MirExpr,
}

#[derive(Debug, Clone)]
enum Continuation<'a> {
    Eval {
        expr: &'a MirExpr,
        environment: RcEnv<'a>,
    },
    If {
        consequent: &'a MirExpr,
        alternative: &'a MirExpr,
        environment: RcEnv<'a>,
    },
    EvFun {
        arg: &'a MirExpr,
        environment: RcEnv<'a>,
    },
    Apply {
        func: Rc<MirObj<'a>>,
        environment: RcEnv<'a>,
    },
}

#[derive(Debug, Clone, Default)]
struct Environment<'a> {
    parent: Option<RcEnv<'a>>,
    bindings: Vec<(MirInternedStr, Rc<MirObj<'a>>)>,
}

#[derive(Debug, Clone)]
struct RcEnv<'a>(Rc<Environment<'a>>);

impl<'a> RcEnv<'a> {
    fn find_value(&self, name: MirInternedStr) -> Option<Rc<MirObj<'a>>> {
        if let Some(b) = self.0.bindings.iter().find(|b| b.0 == name) {
            Some(b.1.clone())
        } else if let Some(p) = &self.0.parent {
            p.find_value(name)
        } else {
            None
        }
    }

    fn with_value<'b: 'a>(self, name: MirInternedStr, value: Rc<MirObj<'b>>) -> RcEnv<'a> {
        RcEnv(Rc::new(Environment {
            parent: Some(RcEnv(self.0.clone())),
            bindings: vec![(name, value)],
        }))
    }
}

pub fn run(expr: &MirExpr) -> Result<MirObj, String> {
    let top_level = RcEnv(Rc::new(Environment::default()));
    let mut stack = Vec::new();
    stack.push(Continuation::Eval {
        expr,
        environment: top_level,
    });
    let mut value = Rc::new(MirObj::Null);
    while let Some(cont) = stack.pop() {
        match cont {
            Continuation::Eval { expr, environment } => {
                eval(expr, environment, &mut stack, &mut value)?;
            }
            Continuation::If {
                consequent,
                alternative,
                environment,
            } => {
                let condition = match *value {
                    MirObj::Bool(t) => t,
                    _ => return Err(format!("expected a bool value, got {:?}", value.clone())),
                };
                stack.push(Continuation::Eval {
                    expr: if condition { consequent } else { alternative },
                    environment,
                })
            }
            Continuation::EvFun { arg, environment } => {
                stack.push(Continuation::Apply {
                    func: value.clone(),
                    environment: environment.clone(),
                });
                stack.push(Continuation::Eval {
                    expr: arg,
                    environment,
                });
            }
            Continuation::Apply { func, environment } => {
                match &*func.clone() {
                    MirObj::Lambda(l) => {
                        let new_env = environment.with_value(l.arg, value.clone());
                        stack.push(Continuation::Eval { expr: l.body, environment: new_env })
                    }
                    MirObj::StatePrim(_) | MirObj::PurePrim(_) => todo!(),
                    _ => return Err(format!("cannot apply {:?}", func))
                }
            }
        }
    }
    Ok(Rc::try_unwrap(value).unwrap())
}

fn eval<'a, 'b>(
    expr: &'a MirExpr,
    environment: RcEnv<'a>,
    stack: &'b mut Vec<Continuation<'a>>,
    value: &'b mut Rc<MirObj<'a>>,
) -> Result<(), String> {
    match expr {
        MirExpr::Lambda(l) => {
            *value = Rc::new(MirObj::Lambda(Box::new(Lambda {
                env: environment,
                arg: l.arg,
                body: &l.body,
            })));
        }
        MirExpr::If(if_) => {
            let If {
                condition,
                consequent,
                alternative,
            } = &**if_;
            stack.push(Continuation::If {
                consequent,
                alternative,
                environment: environment.clone(),
            });
            stack.push(Continuation::Eval {
                expr: condition,
                environment,
            });
        }
        MirExpr::Apply(ap) => {
            let Apply { func, arg } = &**ap;
            stack.push(Continuation::EvFun {
                arg,
                environment: environment.clone(),
            });
            stack.push(Continuation::Eval {
                expr: func,
                environment,
            });
        }
        MirExpr::PurePrim(pp) => *value = Rc::new(MirObj::PurePrim(*pp)),
        MirExpr::StatePrim(sp) => *value = Rc::new(MirObj::StatePrim(*sp)),
        MirExpr::Literal(l) => {
            *value = Rc::new(match &**l {
                MirLiteral::Bool(b) => MirObj::Bool(*b),
                MirLiteral::Int(i) => MirObj::Int(*i),
                MirLiteral::Null => MirObj::Null,
            });
        }
        MirExpr::Ref(name) => {
            if let Some(v) = environment.find_value(*name).clone() {
                *value = v;
            } else {
                return Err(format!("reference to undefined name {}", name));
            }
        }
        _ => unimplemented!("found {:?}, which should be gone after desugaring", expr),
    }
    Ok(())
}
