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

use crate::mir::{MirExpr, Lambda};

#[derive(Debug, Clone, Default)]
pub struct BasicBlock {
    instrs: Vec<DoLine>,
    jump: Option<Jump>,
}

/// This is a sequence of state monad actions, but where only the most
/// recent result is ever accessible.
///
/// In descriptions below, `S[_]` is `State[Stack, _]`, where `Stack`
/// is the reified stack state.
// TODO I've thought about this a little more, and I'm pretty sure we
//      can get away with each block having only exactly one Transform
//      as instructions. I mean obviously it's theoretically possible
//      but I think in this case it would Just Work™.
#[derive(Debug, Clone)]
pub enum DoLine {
    /// Most general case: the expression is of type `A -> S[B]`.
    Transform(Lambda),
    /// Type `S[B]`
    Action(MirExpr),
    /// Type `A -> B`
    Map(Lambda),
    /// Type `B`
    Pure(MirExpr),
}

impl DoLine {
    fn to_mir_expr(&self) -> MirExpr {
        todo!()
    }
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

impl Cfg {
    pub fn add_block(&mut self) -> BlockId {
        self.blocks.push(BasicBlock { instrs: vec![], jump: None });
        self.blocks.len() - 1
    }

    pub fn add_instr(&mut self, instr: DoLine) {
        self.blocks[self.current_block].instrs.push(instr);
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
