// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use rustc::ty::TyCtxt;
use rustc::mir::repr::*;
use rustc::mir::transform::{MirPass, MirSource, Pass};
// use rustc_data_structures::indexed_vec::{Idx, IndexVec};
use rustc::mir::visit::{Visitor, LvalueContext};
use std::collections::HashMap;
use std::collections::hash_map::Entry;

pub struct MoveUpPropagation;

impl<'tcx> MirPass<'tcx> for MoveUpPropagation {
    fn run_pass<'a>(&mut self,
                    _tcx: TyCtxt<'a, 'tcx, 'tcx>,
                    _src: MirSource,
                    mir: &mut Mir<'tcx>) {
        let tuc = TempDefUseFinder::new(mir);
        let candidates = tuc.uses.iter().filter(|&(_, ref uses)| uses.len() == 1);
        for (&tmp, _) in candidates {
            // do something
            debug!("{:?} has only one use!", tmp);
            if let Some(v) = tuc.defs.get(&tmp) {
                debug!("{:?} has {} defs", tmp, v.len());
            } else {
                debug!("we didn't have any defs for {:?}?", tmp);
            }

        }
        // for bb in mir.basic_blocks_mut() {
        //     for (i, stmt) in bb.statements().iter().enumerate() {
        //         if let Statement(_,
        //             StatementKind(Assign(lval, TempDecl(tmpId)))) = stmt {
        //             try_optimze(i, bb);
        //         }
        //     }
        // }
    }
}

// enum StopCondition {
//     DontStop,
//     TempBorrowed,
//     TempUsed,
//     FoundDef,
// }

// fn try_optimze(stmt_idx: usize, bb: BasicBlock) {
//     walk_backwards(stmt_idx, bb);
// }

// fn walk_backwards(stmt_idx: usize, bb: BasicBlock) -> StopCondition {
//     let bail = StopCondition::DontStop;
//     while stmt_idx >= 0 and StopCondition::DontStop == bail {
//         bail = visit(bb.statements[stmt_idx]);
//         stmt_idx -= 1;
//     }
//     if stmt_idx == -1 {
//         for p in mir.predecessors_for(bb) {
//             bail = walk_backwards(p.statements.len()-1, p)
//             if bail { break; }
//         }
//     }
//     bail
// }

// fn walk_forwards(stmt_idx: usize, bb: BasicBlock) -> StopCondition {
//     let bail = StopCondition::DontStop;
//     while stmt_idx < bb.statements.len() and StopCondition::DontStop == bail {
//         bail = visit(bb.statements[stmt_idx]);
//         stmt_idx -= 1;
//     }
//     if stmt_idx == bb.statements.len() - 1 {
//         for p in mir.predecessors_for(bb) {
//             bail = walk_forwards(0, p)
//             if bail { break; }
//         }
//     }
//     bail
// }

// fn visit(stmt: Statement) -> StopCondition {
//     StopCondition::Success
// }

impl Pass for MoveUpPropagation {}

struct StatementLocation {
    basic_block: BasicBlock,
    statement_index: usize,
}

struct TempDefUseFinder {
    pub defs: HashMap<Temp, Vec<StatementLocation>>,
    pub uses: HashMap<Temp, Vec<StatementLocation>>,
    curr_basic_block: BasicBlock,
    statement_index: usize,
}

impl TempDefUseFinder {
    fn new(mir: &Mir) -> Self {
        let mut tuc = TempDefUseFinder {
            defs: HashMap::new(),
            uses: HashMap::new(),
            curr_basic_block: START_BLOCK,
            statement_index: 0,
        };
        tuc.visit_mir(mir);
        tuc
    }
}
impl<'a> Visitor<'a> for TempDefUseFinder {
    fn visit_basic_block_data(&mut self, block: BasicBlock, data: &BasicBlockData<'a>) {
        self.curr_basic_block = block;
        self.statement_index = 0;
        self.super_basic_block_data(block, data);
    }
    fn visit_statement(&mut self, block: BasicBlock, statement: &Statement<'a>) {
        self.super_statement(block, statement);
        self.statement_index += 1;
    }
    fn visit_lvalue(&mut self, lvalue: &Lvalue<'a>, context: LvalueContext) {
        match context {
            LvalueContext::Store => {
                if let &Lvalue::Temp(tmp_id) = lvalue {
                    self.defs.entry(tmp_id).or_insert(vec![]).push(StatementLocation {
                        basic_block: self.curr_basic_block,
                        statement_index: self.statement_index,
                    });
                }
            }
            // LvalueContext::Call => {},
            // LvalueContext::Drop => {},
            // LvalueContext::Inspect => {},
            // LvalueContext::Borrow { region: Region, kind: BorrowKind },
            // LvalueContext::Slice { from_start: usize, from_end: usize },
            // LvalueContext::Projection => {},
            // LvalueContext::Consume => {},
            _ => {
                if let &Lvalue::Temp(tmp_id) = lvalue {
                    self.uses.entry(tmp_id).or_insert(vec![]).push(StatementLocation {
                        basic_block: self.curr_basic_block,
                        statement_index: self.statement_index,
                    });
                }
            }
        }
        self.super_lvalue(lvalue, context);
    }
}

// one potential problem with only looking at
// statements is what if the temporary I'm eliminating
// is used in a terminator?