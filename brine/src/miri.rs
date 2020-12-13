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

use crate::mir::{Apply, If, MirExpr, MirLiteral, Primitive, MirInternedStr};
use saltwater_parser::InternedStr;
use std::rc::Rc;

/// A Mir runtime object
#[derive(Debug, Clone)]
pub enum Obj<'a> {
    Bool(bool),
    Int(i64),
    Null,
    Lambda(Box<Lambda<'a>>),
    CurriedPrimitive(CurriedPrimitive<'a>),
    Cons(Rc<Obj<'a>>, Rc<Obj<'a>>),
}

#[derive(Debug, Clone)]
pub struct Lambda<'a> {
    env: RcEnv<'a>,
    arg: MirInternedStr,
    body: &'a MirExpr,
}

#[derive(Debug, Clone)]
pub struct CurriedPrimitive<'a> {
    primitive: Primitive,
    args: Vec<Rc<Obj<'a>>>
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
        func: Rc<Obj<'a>>,
        environment: RcEnv<'a>,
    },
}

#[derive(Debug, Clone, Default)]
struct Environment<'a> {
    parent: Option<RcEnv<'a>>,
    bindings: Vec<(MirInternedStr, Rc<Obj<'a>>)>,
}

#[derive(Debug, Clone)]
struct RcEnv<'a>(Rc<Environment<'a>>);

impl<'a> RcEnv<'a> {
    fn find_value(&self, name: MirInternedStr) -> Option<Rc<Obj<'a>>> {
        if let Some(b) = self.0.bindings.iter().find(|b| b.0 == name) {
            Some(b.1.clone())
        } else if let Some(p) = &self.0.parent {
            p.find_value(name)
        } else {
            None
        }
    }

    fn with_value<'b: 'a>(self, name: MirInternedStr, value: Rc<Obj<'b>>) -> RcEnv<'a> {
        RcEnv(Rc::new(Environment {
            parent: Some(RcEnv(self.0.clone())),
            bindings: vec![(name, value)],
        }))
    }
}

pub fn run(expr: &MirExpr) -> Result<Obj, String> {
    let top_level = RcEnv(Rc::new(Environment::default()));
    let mut stack = Vec::new();
    stack.push(Continuation::Eval {
        expr,
        environment: top_level,
    });
    let mut value = Rc::new(Obj::Null);
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
                    Obj::Bool(t) => t,
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
                    Obj::Lambda(l) => {
                        let new_env = environment.with_value(l.arg, value.clone());
                        stack.push(Continuation::Eval { expr: l.body, environment: new_env })
                    }
                    Obj::CurriedPrimitive(p) => value = apply_primitive(p, value)?,
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
    value: &'b mut Rc<Obj<'a>>,
) -> Result<(), String> {
    match expr {
        MirExpr::Lambda(l) => {
            *value = Rc::new(Obj::Lambda(Box::new(Lambda {
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
        MirExpr::Primitive(sp) => *value = Rc::new(Obj::CurriedPrimitive(CurriedPrimitive { primitive: *sp, args: vec![] })),
        MirExpr::Literal(l) => {
            *value = Rc::new(match &**l {
                MirLiteral::Bool(b) => Obj::Bool(*b),
                MirLiteral::Int(i) => Obj::Int(*i),
                MirLiteral::Null => Obj::Null,
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


#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ObjType {
    Bool,
    Int,
    Null,
    Lambda,
    CurriedPrimitive,
    Cons,
    Any,
}

impl ObjType {
    fn check_type<'a>(&self, obj: &Obj<'a>) -> bool {
        match (self, obj) {
            (ObjType::Any, _) => true,
            (ObjType::Bool, Obj::Bool(_)) => true,
            (ObjType::Int, Obj::Int(_)) => true,
            (ObjType::Null, Obj::Null) => true,
            (ObjType::Lambda, Obj::Lambda(_)) => true,
            (ObjType::CurriedPrimitive, Obj::CurriedPrimitive(_)) => true,
            (ObjType::Cons, Obj::Cons(_, _)) => true,
            _ => false,
        }
    }
}

fn get_int(obj: &Obj) -> i64 {
    match obj {
        Obj::Int(i) => *i,
        _ => panic!("expected int, got {:?}", obj),
    }
}

fn get_bool(obj: &Obj) -> bool {
    match obj {
        Obj::Bool(b) => *b,
        _ => panic!("expected bool, got {:?}", obj),
    }
}

fn get_pair<'a, 'b>(obj: &'a Obj<'b>) -> (Rc<Obj<'b>>, Rc<Obj<'b>>) {
    match obj {
        Obj::Cons(car, cdr) => (car.clone(), cdr.clone()),
        _ => panic!("expected pair, got {:?}", obj),
    }
}

fn apply_primitive<'a>(prim: &CurriedPrimitive<'a>, arg: Rc<Obj<'a>>) -> Result<Rc<Obj<'a>>, String> {
    let expected_args = match prim.primitive {
        Primitive::Plus => &[ObjType::Int, ObjType::Int][..],
        Primitive::Minus => &[ObjType::Int, ObjType::Int][..],
        Primitive::Times => &[ObjType::Int, ObjType::Int][..],
        Primitive::Div => &[ObjType::Int, ObjType::Int][..],
        Primitive::Mod => &[ObjType::Int, ObjType::Int][..],
        Primitive::Neg => &[ObjType::Bool][..],
        Primitive::And => &[ObjType::Bool, ObjType::Bool][..],
        Primitive::Or => &[ObjType::Bool, ObjType::Bool][..],
        Primitive::Xor => &[ObjType::Bool, ObjType::Bool][..],
        Primitive::Cons => &[ObjType::Any, ObjType::Any][..],
        Primitive::Car => &[ObjType::Cons][..],
        Primitive::Cdr => &[ObjType::Cons][..],
        Primitive::Eq => &[ObjType::Int, ObjType::Int][..],
        Primitive::Gt => &[ObjType::Int, ObjType::Int][..],
        Primitive::Ge => &[ObjType::Int, ObjType::Int][..],
        Primitive::Lt => &[ObjType::Int, ObjType::Int][..],
        Primitive::Le => &[ObjType::Int, ObjType::Int][..],
        Primitive::BoolToInt => &[ObjType::Bool][..],
        p => panic!("got primitive {:?}, which should have been desugared", p),
    };
    let mut args = prim.args.clone();
    let arg_pos = prim.args.len();
    let expected_type = expected_args[arg_pos];
    if !expected_type.check_type(&*arg) {
        return Err(format!("primitive {:?}: expected type {:?}, but got {:?}", prim.primitive, expected_type, &arg));
    }
    args.push(arg);
    if args.len() < expected_args.len() {
        return Ok(Rc::new(Obj::CurriedPrimitive(CurriedPrimitive {
            primitive: prim.primitive,
            args,
        })));
    }
    let val = match prim.primitive {
        Primitive::Plus => Obj::Int(get_int(&*args[0]) + get_int(&*args[1])),
        Primitive::Minus => Obj::Int(get_int(&*args[0]) - get_int(&*args[1])),
        Primitive::Times => Obj::Int(get_int(&*args[0]) * get_int(&*args[1])),
        Primitive::Div => Obj::Int(get_int(&*args[0]) / get_int(&*args[1])),
        Primitive::Mod => Obj::Int(get_int(&*args[0]) % get_int(&*args[1])),
        Primitive::Neg => Obj::Bool(!get_bool(&*args[0])),
        Primitive::And => Obj::Bool(get_bool(&*args[0]) && get_bool(&*args[1])),
        Primitive::Or => Obj::Bool(get_bool(&*args[0]) || get_bool(&*args[1])),
        Primitive::Xor => Obj::Bool(get_bool(&*args[0]) == get_bool(&*args[1])),
        Primitive::Cons => Obj::Cons(args[0].clone(), args[1].clone()),
        Primitive::Car => Obj::clone(&*get_pair(&*args[0]).0),
        Primitive::Cdr => Obj::clone(&*get_pair(&*args[0]).1),
        Primitive::Eq => Obj::Bool(get_int(&*args[0]) == get_int(&*args[1])),
        Primitive::Lt => Obj::Bool(get_int(&*args[0]) < get_int(&*args[1])),
        Primitive::Le => Obj::Bool(get_int(&*args[0]) <= get_int(&*args[1])),
        Primitive::Gt => Obj::Bool(get_int(&*args[0]) > get_int(&*args[1])),
        Primitive::Ge => Obj::Bool(get_int(&*args[0]) >= get_int(&*args[1])),
        Primitive::BoolToInt => Obj::Int(i64::from(get_bool(&*args[0]))),
        p => panic!("got primitive {:?}, which should have been desugared", p),
    };
    Ok(Rc::new(val))
}
