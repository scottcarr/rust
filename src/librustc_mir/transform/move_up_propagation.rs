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
use rustc_data_structures::graph_algorithms::Graph;
use rustc_data_structures::graph_algorithms::dominators::{dominators, Dominators};
use rustc_data_structures::graph_algorithms::transpose::TransposedGraph;

pub struct MoveUpPropagation;

impl<'tcx> MirPass<'tcx> for MoveUpPropagation {
    fn run_pass<'a>(&mut self,
                    tcx: TyCtxt<'a, 'tcx, 'tcx>,
                    src: MirSource,
                    mir: &mut Mir<'tcx>) {
        let node_id = src.item_id();
        let node_path = tcx.item_path_str(tcx.map.local_def_id(node_id));
        debug!("move-up-propagation on {:?}", node_path);
        let mir_clone = mir.clone();
        let transposed_mir = TransposedGraph::new(mir_clone);
        let dominators = dominators(&transposed_mir);
        let tduf = TempDefUseFinder::new(mir);
        tduf.print(mir);
        let candidates = tduf.lists.iter().filter(|&(tmp, lists)| lists.uses.len() == 1 && lists.defs.len() == 1);
        for (tmp, lists) in candidates {
            debug!("{:?} is a candidate", tmp);
            // check if
            // -- Def: L = foo 
            // is post dominated by
            // -- Use: bar = ... L ...           
            // if so,
            // replace Def wit
            // -- Repl: bar = ... foo ...

            let ldef = lists.defs.first();
            let luse = lists.uses.first();
            if ldef.is_post_dominated_by(luse) {
                // do something
            }



            // I wonder if there should be a NOP to preserve indexes ...
            
        }
    
        // let candidates = tduf.uses.iter().filter(|&(_, ref uses)| uses.len() == 1);
        // // for (&tmp, _) in candidates {
        // //     // do something
        // //     debug!("{:?} has only one use!", tmp);
        // //     if let Some(v) = tduf.defs.get(&tmp) {
        // //         debug!("{:?} has {} defs", tmp, v.len());
        // //     } else {
        // //         debug!("we didn't have any defs for {:?}?", tmp);
        // //     }
        // // }
        // let c = candidates.filter(|&(tmp, _)| { 
        //     if let Some(defs) = tduf.defs.get(tmp) {
        //         defs.len() == 1
        //     } else {
        //         false
        //     }
        // });
    }
}

impl Pass for MoveUpPropagation {}

#[derive(Debug)]
struct UseDefLocation {
    basic_block: BasicBlock,
    inner_location: InnerLocation,
}
impl UseDefLocation {
    fn print(&self, mir: &Mir) {
        let ref bb = mir[self.basic_block];
        match self.inner_location {
            InnerLocation::StatementIndex(idx) => {
                debug!("{:?}", bb.statements[idx]);
            },
            InnerLocation::Terminator => {
                debug!("{:?}", bb.terminator);
            }
        }
    }
    fn is_post_dominated_by(&self, other: &Self, post_dominators: &Dominators<Mir>) -> bool {
        if self.basic_block == other.basic_block {
            match (&self.inner_location, &other.inner_location) {
                // Assumptions: Terminator post dominates all statements
                // Terminator does not post dominate itself
                (&InnerLocation::StatementIndex(_), &InnerLocation::Terminator) => { true }
                (&InnerLocation::Terminator, &InnerLocation::Terminator) => { false },
                (&InnerLocation::Terminator, &InnerLocation::StatementIndex(_)) => { false }
                (&InnerLocation::StatementIndex(self_idx), &InnerLocation::StatementIndex(other_idx)) => {
                    self_idx < other_idx
                }       
            }
        } else { // self.basic_block != other.basic_block
            post_dominators.is_dominated_by(self.basic_block, other.basic_block)
        }
    }
}

#[derive(Debug)]
enum InnerLocation {
    StatementIndex(usize),
    Terminator,
}

struct DefUseLists {
    pub defs: Vec<UseDefLocation>,
    pub uses: Vec<UseDefLocation>,
}

impl DefUseLists {
    fn new() -> Self {
        DefUseLists{
            uses: vec![],
            defs: vec![],
        }
    }
}

struct TempDefUseFinder {
    pub lists: HashMap<Temp, DefUseLists>,
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
            lists: HashMap::new(),
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
        match lvalue {
            &Lvalue::Temp(tmp_id) => {
                let loc = if self.is_in_terminator {
                    InnerLocation::Terminator
                } else {
                    InnerLocation::StatementIndex(self.statement_index)
                };
                let ent = UseDefLocation {
                    basic_block: self.curr_basic_block,
                    inner_location: loc,
                };
                match self.kind {
                    AccessKind::Def => self.lists.entry(tmp_id).or_insert(DefUseLists::new()).defs.push(ent),
                    AccessKind::Use => self.lists.entry(tmp_id).or_insert(DefUseLists::new()).uses.push(ent),
                };
            }
            _ => {}
        }
    }
    fn print(&self, mir: &Mir) {
        for (k, ref v) in self.lists.iter() {
            debug!("{:?} uses:", k);
            debug!("{:?}", v.uses);
            // this assertion was wrong
            // you can have an unused temporary, ex: the result of a call is never used
            //assert!(v.uses.len() > 0); // every temp should have at least one use
            v.uses.iter().map(|e| UseDefLocation::print(&e, mir)).count();
        }
        for (k, ref v) in self.lists.iter() {
            debug!("{:?} defs:", k);
            debug!("{:?}", v.defs);
            assert!(v.defs.len() > 0); // every temp should have at least one def
            v.defs.iter().map(|e| UseDefLocation::print(&e, mir)).count();
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
