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
use rustc_data_structures::indexed_vec::{Idx};
use rustc::mir::visit::{Visitor, LvalueContext};
use std::collections::{HashMap, HashSet};
//use std::collections::hash_map::Entry;
use rustc_data_structures::tuple_slice::TupleSlice;
//use rustc_data_structures::control_flow_graph::ControlFlowGraph;
//use rustc_data_structures::control_flow_graph::dominators::{dominators, Dominators};
//use rustc_data_structures::control_flow_graph::transpose::TransposedGraph;
use rustc_data_structures::bitvec::BitVector;
//use rustc_data_structures::control_flow_graph::reference;


// TODO get rid of post-dominators and instead use the notion of "could not read x"
// if we have Def tmp0 = ...
// and Use x = tmp0
// it is safe to apply the optimization if
// for all paths that begin at Def and end at some exit
// the path p goes through the Use or the value of x is not read
// on path p, tmp0 cannot be read or borrowed, borrowing x counts as a read
// we consider all calls to potentially read x

pub struct MoveUpPropagation;

impl<'tcx> MirPass<'tcx> for MoveUpPropagation {
    fn run_pass<'a>(&mut self,
                    tcx: TyCtxt<'a, 'tcx, 'tcx>,
                    src: MirSource,
                    mir: &mut Mir<'tcx>) {
        let node_id = src.item_id();
        let node_path = tcx.item_path_str(tcx.map.local_def_id(node_id));
        debug!("move-up-propagation on {:?}", node_path);

        // let post_dominators_res = {
        //     let mir_e = MirWithExit::new(mir);
        //     let exit = mir_e.exit_node.clone();
        //     let tgraph = TransposedGraph::with_start(mir_e, exit);
        //     dominators(&tgraph)
        // };

        // let post_dominators = match post_dominators_res {
        //     Ok(pdoms) => pdoms,
        //     Err(_) => return, // we cant do the optimization when finding the post dominators fails
        // };

        let tduf = TempDefUseFinder::new(mir);
        tduf.print(mir);
        
        let mut old_2_new = HashMap::new();
        let mut dead = HashMap::new();
        for tmp in tduf.get_temps_that_satisfy(mir) {
            if let Ok((use_bb, use_idx)) = tduf.get_one_use_as_idx(&tmp) {
                if let Ok((def_bb, def_idx)) = tduf.get_one_def_as_idx(&tmp) {
                    let bb_mut = mir.basic_blocks();
                    let StatementKind::Assign(ref use_lval, _) = bb_mut[use_bb]
                                                                    .statements[use_idx].kind;
                    let StatementKind::Assign(_, ref def_rval) = bb_mut[def_bb]
                                                                    .statements[def_idx].kind;
                    let new_statement = StatementKind::Assign(use_lval.clone(), def_rval.clone());
                    let num_statements = bb_mut[use_bb].statements.len();
                    old_2_new.entry(def_bb)
                                .or_insert(HashMap::new())
                                .insert(def_idx, new_statement);
                    dead.entry(use_bb)
                        .or_insert(BitVector::new(num_statements))
                        .insert(use_idx);
                }
            }
        }
        debug!("we're going to do {:?} replacements", dead.iter().len());


        //for &(_, lists) in work_list.iter() {
        // for &(luse, ldef) in work_list.iter() {
        //     // let ldef = lists.defs.first().expect("we already checked the list had one element?");
        //     // let luse = lists.uses.first().expect("we already checked the list had one element?");
        //     if let InnerLocation::StatementIndex(use_idx) = luse.inner_location {
        //         if let InnerLocation::StatementIndex(def_idx) = ldef.inner_location {
        //             let bb_mut = mir.basic_blocks();
        //             let StatementKind::Assign(ref use_lval, _) = bb_mut[luse.basic_block]
        //                                                          .statements[use_idx].kind;
        //             let StatementKind::Assign(_, ref def_rval) = bb_mut[ldef.basic_block]
        //                                                          .statements[def_idx].kind;
        //             let new_statement = StatementKind::Assign(use_lval.clone(), def_rval.clone());
        //             let num_statements = bb_mut[luse.basic_block].statements.len();
        //             old_2_new.entry(ldef.basic_block)
        //                      .or_insert(HashMap::new())
        //                      .insert(def_idx, new_statement);
        //             dead.entry(luse.basic_block)
        //                 .or_insert(BitVector::new(num_statements))
        //                 .insert(use_idx);
        //             continue;
        //         }
        //     }
        //     panic!("We should have already checked for this");
        // }

        {
            let bbs = mir.basic_blocks_mut();
            for (bb, repls) in old_2_new {
                let new_stmts: Vec<_> = bbs[bb].statements
                                                .iter()
                                                .enumerate()
                                                .map(|(stmt_idx, orig_stmt)| {
                    if let Some(repl) = repls.get(&stmt_idx) {
                        let repl_stmt = Statement { kind: repl.clone(),
                                                    source_info: orig_stmt.source_info,
                        };
                        debug!("replacing {:?} with {:?}", orig_stmt, repl_stmt);
                        repl_stmt
                    } else {
                        //debug!("repl idx: {:?} didnt match {:?}", idx, stmt_idx);
                        orig_stmt.clone()
                    }
                }).collect();
                bbs[bb] = BasicBlockData {
                    statements: new_stmts,
                    terminator: bbs[bb].terminator.clone(),
                    is_cleanup: bbs[bb].is_cleanup,
                };
            }

            for (bb, dead_idxs) in dead {
                let mut idx_cnt = 0;
                bbs[bb].statements.retain(|_| {
                    let dead = dead_idxs.contains(idx_cnt);
                    idx_cnt += 1;
                    !dead
                });
            }
        }
    }
}

fn get_next_locs(curr: UseDefLocation, mir: &Mir) -> Vec<UseDefLocation> {
    match curr.inner_location {
        InnerLocation::Terminator => {
            mir.basic_blocks()[curr.basic_block].terminator().successors().iter().map(|&s| {
                UseDefLocation {
                    basic_block: s,
                    inner_location: InnerLocation::StatementIndex(0),
                }
            }).collect()
        }
        InnerLocation::StatementIndex(idx) => {
            if idx + 1 < mir.basic_blocks()[curr.basic_block].statements.len() {
                vec![UseDefLocation{
                    basic_block: curr.basic_block,
                    inner_location: InnerLocation::StatementIndex(idx + 1),
                }]
            } else {
                let next = UseDefLocation{ basic_block: curr.basic_block,
                                           inner_location: InnerLocation::Terminator,
                };
                get_next_locs(next, mir)
            }
        }
    }
}

// This could more easily be rewritten as a MirVisitor
fn paths_contain_call(start: UseDefLocation,
                      target: UseDefLocation,
                      mir: &Mir,
                      visited: &mut BitVector)
                      -> bool {
    //   walk the paths from ldef -> ~ -> luse
    //   make sure there are no calls
    //   there can't be any borrows because we know luse is the only use
    //   (because we checked before)
    if start == target {
        false
    } else {
        // check for out stopping condition,
        // if we do not stop, go to the next location
        if let TerminatorKind::Call {..} = mir.basic_blocks()[start.basic_block].terminator().kind {
            true
        } else {
            let mut any = false;
            for &s in get_next_locs(start, mir).iter() {
                if !visited.contains(s.basic_block.index()) {
                    visited.insert(s.basic_block.index());
                    any |= paths_contain_call(s, target, mir, visited);
                }
            }
            any
        }
    }
}

// fn any_funny_business() -> bool {

//     if lists.uses.len() == 1 && lists.defs.len() == 1 {
//     let ldef = match lists.defs.first() {
//         Some(x) => x,
//         None => panic!("we already checked the len?!?"),
//     };
//     let luse = match lists.uses.first() {
//         Some(x) => x,
//         None => panic!("we already checked the len?!?"),
//     };
//     debug!("the combindation of:");
//     luse.print(mir);
//     debug!("and:");
//     ldef.print(mir);
//     debug!("is a move up candidate");
    
//     // IF
//     // -- Def: L = foo
//     // is post dominated by
//     // -- Use: bar = ... L ...
//     // AND
//     // on the path(s) from Def -> ~ -> Use
//     // there are no calls
//     // THEN,
//     // replace Def wit
//     // -- Repl: bar = ... foo ...

//     let mut visited = BitVector::new(mir.basic_blocks().len());
//     if !ldef.is_post_dominated_by(luse, post_dominators) {
//         return true;
//     };

//     if paths_contain_call(*ldef, *luse, mir, &mut visited) {
//         true;
//     }

//     // we really only know how to replace statements for now ...
//     if let InnerLocation::Terminator = ldef.inner_location {
//         return true;
//     }
//     if let InnerLocation::Terminator = luse.inner_location {
//         return true;
//     }

//     //   check if the luse is like: foo = ldef
//     //   if it's more compiled than that, we give up. for now ...
//     if let InnerLocation::StatementIndex(idx) = luse.inner_location {
//         match mir.basic_blocks()[luse.basic_block].statements[idx].kind {
//             StatementKind::Assign(_, ref rval) => {
//                 if let &Rvalue::Use(Operand::Consume(Lvalue::Temp(rtmp))) = rval {
//                     if rtmp == tmp { return false; };
//                 }
//             }
//         }
//     };

//     return true;
// }

impl Pass for MoveUpPropagation {}

#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash)]
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
    // fn is_post_dominated_by(&self, other: &Self, post_dominators: &Dominators<BasicBlock>) -> bool {
    //     if self.basic_block == other.basic_block {
    //         match (&self.inner_location, &other.inner_location) {
    //             // Assumptions: Terminator post dominates all statements
    //             // Terminator does not post dominate itself
    //             (&InnerLocation::StatementIndex(_), &InnerLocation::Terminator) => { true }
    //             (&InnerLocation::Terminator, &InnerLocation::Terminator) => { false },
    //             (&InnerLocation::Terminator, &InnerLocation::StatementIndex(_)) => { false }
    //             (&InnerLocation::StatementIndex(self_idx),
    //              &InnerLocation::StatementIndex(other_idx)) => {
    //                 self_idx < other_idx
    //             }
    //         }
    //     } else { // self.basic_block != other.basic_block
    //         post_dominators.is_dominated_by(self.basic_block, other.basic_block)
    //     }
    // }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash)]
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

struct TempDefUseFinder<'a> {
    pub lists: HashMap<Temp, DefUseLists>,
    pub is_borrowed: HashSet<Lvalue<'a>>,
    curr_basic_block: BasicBlock,
    statement_index: usize,
    kind: AccessKind,
    is_in_terminator: bool,
}

enum AccessKind {
    Def,
    Use,
}

enum GetOneErr {
    NotAStatement,
    NotOne,
}

impl<'a> TempDefUseFinder<'a> {
    fn new(mir: &Mir<'a>) -> Self {
        let mut tuc = TempDefUseFinder {
            lists: HashMap::new(),
            is_borrowed: HashSet::new(),
            curr_basic_block: START_BLOCK,
            statement_index: 0,
            kind: AccessKind::Def, // will get updated when we see an assign
            is_in_terminator: false,
        };
        tuc.visit_mir(mir);
        tuc
    }
    fn add_to_map_if_temp(&mut self,
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
                    AccessKind::Def => self.lists.entry(tmp_id)
                                                 .or_insert(DefUseLists::new())
                                                 .defs
                                                 .push(ent),
                    AccessKind::Use => self.lists.entry(tmp_id)
                                                 .or_insert(DefUseLists::new())
                                                 .uses
                                                 .push(ent),
                };
            }
            _ => {}
        }
    }
    fn get_one_use_as_idx(&self, temp: &Temp) -> Result<(BasicBlock, usize), GetOneErr> {
        if self.lists[temp].uses.len() != 1 {
            return Err(GetOneErr::NotOne);
        };
        let use_loc = self.lists[temp].uses.first().unwrap();
        let use_bb = use_loc.basic_block;
        let use_idx = match use_loc.inner_location {
            InnerLocation::StatementIndex(idx) => idx,
            _ => return Err(GetOneErr::NotAStatement),
        };
        Ok((use_bb, use_idx)) 
    }
    fn get_one_def_as_idx(&self, temp: &Temp) -> Result<(BasicBlock, usize), GetOneErr> {
        if self.lists[temp].defs.len() != 1 {
            return Err(GetOneErr::NotOne);
        };
        let def_loc = self.lists[temp].defs.first().unwrap();
        let def_bb = def_loc.basic_block;
        let def_idx = match def_loc.inner_location {
            InnerLocation::StatementIndex(idx) => idx,
            _ => return Err(GetOneErr::NotAStatement),
        };
        Ok((def_bb, def_idx))
    }
    fn get_temps_that_satisfy(&self, mir: &Mir) -> Vec<Temp> {
        self.lists.iter().filter(|&(_, lists)| {
            lists.uses.len() == 1 && lists.defs.len() == 1     
        }).map(|(&tmp, _)| {
            tmp
        }).filter(|tmp| {
            if let Ok((use_bb, use_idx)) = self.get_one_use_as_idx(&tmp) {
                // this handles the constraint: DEST is not borrowed (currently: not borrowed ever)
                let StatementKind::Assign(ref dest, _) = mir.basic_blocks()[use_bb].statements[use_idx].kind;
                return !self.is_borrowed.contains(&dest);
            }
            false
        }).filter(|tmp| {
            // also check all paths starting from Def(tmp = ...) to "exit" for our condition
            let mut visited = BitVector::new(mir.basic_blocks().len());
            let ldef = self.lists[tmp].defs.first().unwrap();
            let luse = self.lists[tmp].uses.first().unwrap();
            !paths_contain_call(*luse, *ldef, mir, &mut visited)
        }).collect()
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
            // this may be too strict? maybe the def was optimized out?
            //assert!(v.defs.len() > 0); // every temp should have at least one def
            v.defs.iter().map(|e| UseDefLocation::print(&e, mir)).count();
        }
    }
}
impl<'a> Visitor<'a> for TempDefUseFinder<'a> {
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
        if let LvalueContext::Borrow{ .. } = context {
            self.is_borrowed.insert(lvalue.clone());
        };
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
