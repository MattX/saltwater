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

use crate::mir::MirExpr;

#[derive(Debug, Clone)]
pub struct BasicBlock {
    expr: MirExpr,
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

#[derive(Debug, Clone, Default)]
pub struct Cfg {
    blocks: Vec<BasicBlock>,
    return_block_id: Option<BlockId>,
}

impl Cfg {
    pub fn add_block(&mut self, expr: MirExpr) -> BlockId {
        self.blocks.push(BasicBlock { expr, jump: None });
        self.blocks.len() - 1
    }

    pub fn set_jump(&mut self, id: BlockId, jump: Jump) {
        let old = self.blocks[id].jump.replace(jump);
        debug_assert!(old.is_none(), "changing block jump");
    }

    pub fn set_return_block(&mut self, id: BlockId) {
        let old = self.return_block_id.replace(id);
        debug_assert!(old.is_none(), "changing return block ID");
    }
}
