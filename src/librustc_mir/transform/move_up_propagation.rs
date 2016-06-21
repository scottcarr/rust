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
//use std::collections::hash_map::Entry;
use rustc_data_structures::tuple_slice::TupleSlice;

pub struct MoveUpPropagation;

impl<'tcx> MirPass<'tcx> for MoveUpPropagation {
    fn run_pass<'a>(&mut self,
                    _tcx: TyCtxt<'a, 'tcx, 'tcx>,
                    _src: MirSource,
                    mir: &mut Mir<'tcx>) {
        let tduf = TempDefUseFinder::new(mir);
        tduf.print(mir);
        let candidates = tduf.uses.iter().filter(|&(_, ref uses)| uses.len() == 1);
        for (&tmp, _) in candidates {
            // do something
            debug!("{:?} has only one use!", tmp);
            if let Some(v) = tduf.defs.get(&tmp) {
                debug!("{:?} has {} defs", tmp, v.len());
            } else {
                debug!("we didn't have any defs for {:?}?", tmp);
            }
        }
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

#[derive(Debug)]
struct UseDefLocation {
    basic_block: BasicBlock,
    inner_location: StatementIndexOrTerminator,
}
impl UseDefLocation {
    fn print(&self, mir: &Mir) {
        let ref bb = mir[e.basic_block];
        match e.inner_location {
            StatementIndexOrTerminator::StatementIndex(idx) => {
                debug!("{:?}", bb.statements[idx]);
            },
            StatementIndexOrTerminator::Terminator => {
                debug!("{:?}", bb.terminator);
            }
        }
    }
}

#[derive(Debug)]
enum StatementIndexOrTerminator {
    StatementIndex(usize),
    Terminator,
}

struct TempDefUseFinder {
    pub defs: HashMap<Temp, Vec<UseDefLocation>>,
    pub uses: HashMap<Temp, Vec<UseDefLocation>>,
    curr_basic_block: BasicBlock,
    statement_index: usize,
    kind: AccessKind,
    is_in_terminator: bool,
}

enum AccessKind {
    Def,
    Use,
}

impl TempDefUseFinder {
    fn new(mir: &Mir) -> Self {
        let mut tuc = TempDefUseFinder {
            defs: HashMap::new(),
            uses: HashMap::new(),
            curr_basic_block: START_BLOCK,
            statement_index: 0,
            kind: AccessKind::Def, // not necessarily right but it'll get updated when we see an assign
            is_in_terminator: false,
        };
        tuc.visit_mir(mir);
        tuc
    }
    fn add_to_map_if_temp<'a>(&mut self,
                          lvalue: &Lvalue<'a>) {
        let mut hashmap = match self.kind {
            AccessKind::Def => &mut self.defs,
            AccessKind::Use => &mut self.uses,
        };
        match lvalue {
            &Lvalue::Temp(tmp_id) => {
                let loc = if self.is_in_terminator {
                    StatementIndexOrTerminator::Terminator
                } else {
                    StatementIndexOrTerminator::StatementIndex(self.statement_index)
                };
                hashmap.entry(tmp_id).or_insert(vec![]).push(UseDefLocation {
                    basic_block: self.curr_basic_block,
                    inner_location: loc,
                });
            }
            _ => {}
        }
    }
    fn print(&self, mir: &Mir) {
        for (k, v) in self.uses.iter() {
            assert!(v.len() > 0); // every temp should have at least one use
            debug!("{:?} uses:", k);
            for e in v { e.print(mir); }
        }
        for (k, v) in self.defs.iter() {
            assert!(v.len() > 0); // every temp should have at least one def
            debug!("{:?} defs:", k);
            for e in v { e.print(mir); }
        }
    }
}
impl<'a> Visitor<'a> for TempDefUseFinder {
    fn visit_basic_block_data(&mut self, block: BasicBlock, data: &BasicBlockData<'a>) {
        self.curr_basic_block = block;
        self.statement_index = 0;
        self.is_in_terminator = false;
        self.super_basic_block_data(block, data);
    }
    fn visit_statement(&mut self, _: BasicBlock, statement: &Statement<'a>) {
        match statement.kind {
            StatementKind::Assign(ref lvalue, ref rvalue) => {
                self.kind = AccessKind::Def;
                self.visit_lvalue(lvalue, LvalueContext::Store);
                self.kind = AccessKind::Use;
                self.visit_rvalue(rvalue);
            },
        }
        self.statement_index += 1;
    }
    fn visit_lvalue(&mut self, lvalue: &Lvalue<'a>, context: LvalueContext) {
        self.add_to_map_if_temp(lvalue);
        self.super_lvalue(lvalue, context);
    }
    fn visit_terminator(&mut self, block: BasicBlock, terminator: &Terminator<'a>) {
        self.is_in_terminator = true;
        self.super_terminator(block, terminator);                
    }
    fn visit_terminator_kind(&mut self, block: BasicBlock, kind: &TerminatorKind<'a>) {
        match *kind {
            TerminatorKind::Goto { target } => {
                self.visit_branch(block, target);
            }

            TerminatorKind::If { ref cond, ref targets } => {
                self.kind = AccessKind::Use;
                self.visit_operand(cond);
                for &target in targets.as_slice() {
                    self.visit_branch(block, target);
                }
            }

            TerminatorKind::Switch { ref discr,
                                        adt_def: _,
                                        ref targets } => {
                self.kind = AccessKind::Use;
                self.visit_lvalue(discr, LvalueContext::Inspect);
                for &target in targets {
                    self.visit_branch(block, target);
                }
            }

            TerminatorKind::SwitchInt { ref discr,
                                        ref switch_ty,
                                        ref values,
                                        ref targets } => {
                self.kind = AccessKind::Use;
                self.visit_lvalue(discr, LvalueContext::Inspect);
                self.visit_ty(switch_ty);
                for value in values {
                    self.visit_const_val(value);
                }
                for &target in targets {
                    self.visit_branch(block, target);
                }
            }

            TerminatorKind::Resume |
            TerminatorKind::Return |
            TerminatorKind::Unreachable => {
            }

            TerminatorKind::Drop { ref location,
                                    target,
                                    unwind } => {
                self.kind = AccessKind::Use;
                self.visit_lvalue(location, LvalueContext::Drop);
                self.visit_branch(block, target);
                unwind.map(|t| self.visit_branch(block, t));
            }

            TerminatorKind::DropAndReplace { ref location,
                                                ref value,
                                                target,
                                                unwind } => {
                self.kind = AccessKind::Use;
                self.visit_lvalue(location, LvalueContext::Drop);
                self.visit_operand(value);
                self.visit_branch(block, target);
                unwind.map(|t| self.visit_branch(block, t));
            }

            TerminatorKind::Call { ref func,
                                    ref args,
                                    ref destination,
                                    cleanup } => {
                self.visit_operand(func);
                for arg in args {
                    self.visit_operand(arg);
                }
                if let Some((ref destination, target)) = *destination {
                    self.kind = AccessKind::Def; // this is the whole reason for this function
                    self.visit_lvalue(destination, LvalueContext::Call);
                    self.kind = AccessKind::Use; // this is the whole reason for this function
                    self.visit_branch(block, target);
                }
                cleanup.map(|t| self.visit_branch(block, t));
            }

            TerminatorKind::Assert { ref cond,
                                        expected: _,
                                        ref msg,
                                        target,
                                        cleanup } => {
                self.kind = AccessKind::Use;
                self.visit_operand(cond);
                self.visit_assert_message(msg);
                self.visit_branch(block, target);
                cleanup.map(|t| self.visit_branch(block, t));
            }
        }
    }
}
