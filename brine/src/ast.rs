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

// TODO refactor this to reuse relambda AST features?

#![allow(dead_code)]

use std::fmt::Formatter;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Combinator {
    I,
    K,
    S,
    V,
    D,
    C,
    E,
    Read,
    Reprint,
    Compare(char),
    Dot(char),
}

impl std::fmt::Display for Combinator {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Combinator::I => write!(f, "i"),
            Combinator::K => write!(f, "k"),
            Combinator::S => write!(f, "s"),
            Combinator::V => write!(f, "v"),
            Combinator::D => write!(f, "d"),
            Combinator::C => write!(f, "c"),
            Combinator::E => write!(f, "e"),
            Combinator::Read => write!(f, "@"),
            Combinator::Reprint => write!(f, "|"),
            Combinator::Compare(c) => write!(f, "?{}", c),
            Combinator::Dot(c) => write!(f, ".{}", c),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Application {
    pub func: SyntaxNode,
    pub arg: SyntaxNode,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Abstraction {
    pub variable: String,
    pub body: SyntaxNode,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SyntaxNode {
    Combinator(Combinator),
    Application(Box<Application>),
    Abstraction(Box<Abstraction>),
    Reference(String),
}

impl SyntaxNode {
    pub fn application(func: SyntaxNode, arg: SyntaxNode) -> SyntaxNode {
        SyntaxNode::Application(Box::new(Application { func, arg }))
    }

    pub fn abstraction(variable: String, body: SyntaxNode) -> SyntaxNode {
        SyntaxNode::Abstraction(Box::new(Abstraction { variable, body }))
    }

    pub fn output(&self, max_width: usize) -> String {
        self.do_output(0, max_width).0
    }

    /// Returns a representation of the node, and a boolean indicating whether the representation
    /// contains a line break.
    fn do_output(&self, indent: usize, max_width: usize) -> (String, bool) {
        match self {
            SyntaxNode::Combinator(c) => (format!("{}", c), false),
            SyntaxNode::Application(a) => {
                let (func_repr, func_br) = a.func.do_output(indent + 1, max_width);
                let (arg_repr, arg_br) = a.arg.do_output(indent + 1, max_width);
                let total_len = 1 + func_repr.chars().count() + arg_repr.chars().count();
                if func_br || arg_br || indent + total_len > max_width {
                    (
                        format!("`{}\n{}{}", func_repr, " ".repeat(indent + 1), arg_repr),
                        true,
                    )
                } else {
                    (format!("`{}{}", func_repr, arg_repr), false)
                }
            }
            SyntaxNode::Reference(r) => (format!("${}", r), false),
            SyntaxNode::Abstraction(a) => {
                let new_indent = indent + a.variable.chars().count() + 2;
                let (body_repr, body_br) = a.body.do_output(new_indent, max_width);
                (format!("λ{} {}", a.variable, body_repr), body_br)
            }
        }
    }
}
