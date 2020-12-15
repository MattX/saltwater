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

//! ## Control flow graph
//!
//! We go from C to a mostly-purely-functional CFG representation where
//! every basic block consists of a single Mir expression + a maybe-conditional
//! jump.
//!
//! Each basic block has exactly one (implicit) parameter, just like each line
//! in a do-block has one implicit parameter, the previous result.

use crate::mir::{MirExpr, Lambda, MirInternedStr, Primitive, MirLiteral};

#[derive(Debug, Clone, Default)]
pub struct BasicBlock {
    instr: Option<Lambda>,
    jump: Option<Jump>,
}

pub type BlockId = usize;

#[derive(Debug, Clone)]
pub enum Jump {
    /// Unconditionally jump to the pointed block
    Jmp(BlockId),

    /// If the current value is true, jump to the first block; otherwise,
    /// jump to the second block.
    Br(BlockId, BlockId),
}

#[derive(Debug, Clone)]
pub struct Cfg {
    blocks: Vec<BasicBlock>,
    return_block_id: Option<BlockId>,
    current_block: BlockId,
}

lazy_static! {
    static ref NEXT_BLOCK: MirInternedStr = MirInternedStr::get_or_intern("next_block");
    static ref DISCRIMINANT: MirInternedStr = MirInternedStr::get_or_intern("discriminant");
}

impl Cfg {
    pub fn add_block(&mut self) -> BlockId {
        self.blocks.push(BasicBlock::default());
        self.blocks.len() - 1
    }

    pub fn add_instr(&mut self, instr: Lambda) {
        let old = self.blocks[self.current_block].instr.replace(instr);
        debug_assert!(old.is_none());
    }

    pub fn set_jump(&mut self, jump: Jump) {
        let old = self.blocks[self.current_block].jump.replace(jump);
        debug_assert!(old.is_none(), "changing block jump");
    }

    pub fn set_return_block(&mut self, id: BlockId) {
        let old = self.return_block_id.replace(id);
        debug_assert!(old.is_none(), "changing return block ID");
    }

    pub fn switch_to_block(&mut self, id: BlockId) {
        self.current_block = id;
    }

    pub fn to_mir(&self) -> MirExpr {
        todo!()
    }
}

impl Default for Cfg {
    fn default() -> Self {
        Cfg {
            blocks: vec![BasicBlock::default()],
            return_block_id: None,
            current_block: 0,
        }
    }
}


/// Generates a switch statement, such that when discriminant is
/// `n`, the `n`th expression in `exprs` will be selected.
///
/// If the discriminant is not within `0..exprs.len()` at runtime,
/// behavior is undefined.
fn switch(discriminant: MirExpr, mut exprs: Vec<MirExpr>) -> MirExpr {
    MirExpr::let_(*DISCRIMINANT, discriminant, switch_helper(0, exprs))
}

fn switch_helper(current_pos: i64, mut exprs: Vec<MirExpr>) -> MirExpr {
    let expr = exprs.pop().expect("switch given empty exprs");
    if exprs.is_empty() {
        expr
    } else {
        let discriminant = MirExpr::Ref(*DISCRIMINANT);
        MirExpr::if_(eq_expr(current_pos, discriminant), expr, switch_helper(current_pos + 1, exprs))
    }
}

fn eq_expr(n: i64, e: MirExpr) -> MirExpr {
    MirExpr::apply(MirExpr::apply(MirExpr::Primitive(Primitive::Eq), MirExpr::literal(MirLiteral::Int(n))), e)
}
