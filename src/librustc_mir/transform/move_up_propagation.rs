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
use rustc::mir::visit::{Visitor, LvalueContext};
use std::collections::{HashMap, HashSet};
use rustc_data_structures::tuple_slice::TupleSlice;

// get rid of post-dominators and instead use the notion of "could not read x"
// if we have Def tmp0 = ...
// and Use x = tmp0
// it is safe to apply the optimization if
// for all paths that begin at Def and end at some exit
// the path p goes through the Use, or the value of x is not read
// on path p, tmp0 cannot be read or borrowed, borrowing x counts as a read
// we consider all calls to potentially read x

pub struct MoveUpPropagation;

impl Pass for MoveUpPropagation {}

impl<'tcx> MirPass<'tcx> for MoveUpPropagation {
    fn run_pass<'a>(&mut self,
                    tcx: TyCtxt<'a, 'tcx, 'tcx>,
                    src: MirSource,
                    mir: &mut Mir<'tcx>) {
        let node_id = src.item_id();
        let node_path = tcx.item_path_str(tcx.map.local_def_id(node_id));
        debug!("move-up-propagation on {:?}", node_path);

        let mut opt_counter = 0;
        while let Some((use_bb, use_idx, def_bb, def_idx)) = get_one_optimization(mir) {
            let new_statement_kind = get_replacement_statement(mir,
                                                               use_bb,
                                                               use_idx,
                                                               def_bb,
                                                               def_idx);

            let mut bbs = mir.basic_blocks_mut();
            // replace Def(tmp = ...) with DEST = ...
            let new_def_stmts: Vec<_> = bbs[def_bb].statements
                                                    .iter()
                                                    .enumerate()
                                                    .map(|(stmt_idx, orig_stmt)| {
                if stmt_idx == def_idx {
                    let new_statement = Statement {
                        kind: new_statement_kind.clone(),
                        source_info: orig_stmt.source_info
                    };
                    debug!("replacing: {:?} with {:?}.", orig_stmt, new_statement);
                    new_statement
                } else {
                    orig_stmt.clone()
                }
            }).collect();
            bbs[def_bb] = BasicBlockData {
                statements: new_def_stmts,
                terminator: bbs[def_bb].terminator.clone(),
                is_cleanup: bbs[def_bb].is_cleanup,
            };

            // remove DEST = tmp
            let mut idx_cnt = 0;
            bbs[use_bb].statements.retain(|orig_stmt| {
                let dead = idx_cnt == use_idx;
                idx_cnt += 1;
                if dead {
                    debug!("deleting: {:?}", orig_stmt);
                }
                !dead
            });
            opt_counter += 1;
        }
        debug!("we did {:?} optimizations", opt_counter);
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash)]
struct UseDefLocation {
    basic_block: BasicBlock,
    inner_location: InnerLocation,
}

// impl UseDefLocation {
//     fn print(&self, mir: &Mir) {
//         let ref bb = mir[self.basic_block];
//         match self.inner_location {
//             InnerLocation::StatementIndex(idx) => {
//                 debug!("{:?}", bb.statements[idx]);
//             },
//             InnerLocation::Terminator => {
//                 debug!("{:?}", bb.terminator);
//             }
//         }
//     }
// }

#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash)]
enum InnerLocation {
    StatementIndex(usize),
    Terminator,
}

fn get_replacement_statement<'a>(mir: &Mir<'a>,
                             use_bb: BasicBlock,
                             use_idx: usize,
                             def_bb: BasicBlock,
                             def_idx: usize)
                             -> StatementKind<'a> {
    let bbs = mir.basic_blocks();
    let StatementKind::Assign(ref use_lval, _) = bbs[use_bb]
                                                    .statements[use_idx].kind;
    let StatementKind::Assign(_, ref def_rval) = bbs[def_bb]
                                                    .statements[def_idx].kind;
    StatementKind::Assign(use_lval.clone(), def_rval.clone())
}

fn paths_satisfy_our_condition<'a>(dest: &Lvalue<'a>,
                                   end_statement: &Statement<'a>,
                                   end_block: BasicBlock,
                                   start_block: BasicBlock,
                                   start_index: usize,
                                   start_block_data: &BasicBlockData<'a>)
                                   -> bool {
    let mut upf = UseOnPathFinder {
        dest: dest,
        end_statement: end_statement,
        end_block: end_block,
        found_intermediate_use_of_dest: false,
        found_call: false,
        found_end_statement: false,
    };
    upf.visit_first_block(start_block, start_block_data, start_index);
    upf.found_end_statement && !upf.found_intermediate_use_of_dest && !upf.found_call
}

struct UseOnPathFinder<'a> {
    dest: &'a Lvalue<'a>,
    end_statement: &'a Statement<'a>,
    end_block: BasicBlock,
    found_intermediate_use_of_dest: bool,
    found_call: bool,
    found_end_statement: bool,
}

impl<'a> UseOnPathFinder<'a> {
    fn visit_first_block(&mut self,
                         block: BasicBlock,
                         data: &BasicBlockData<'a>,
                         start_index: usize) {

        for (i, statement) in data.statements.iter().enumerate() {
            if i > start_index {
                self.visit_statement(block, statement);
            }
        }

        if let Some(ref terminator) = data.terminator {
            self.visit_terminator(block, terminator);
        }
    }
}

impl<'a> Visitor<'a> for UseOnPathFinder<'a> {
    fn visit_statement(&mut self, block: BasicBlock, statement: &Statement<'a>) {
        if self.end_block == block && self.end_statement == statement {
            debug!("found end statement!");
            self.found_end_statement = true;
            return; // stop searching
        }
        self.super_statement(block, statement);
    }
    fn visit_lvalue(&mut self, lvalue: &Lvalue<'a>, context: LvalueContext) {
        if lvalue == self.dest {
            debug!("found intermediate use of dest!");
            self.found_intermediate_use_of_dest = true;
            return; // stop searching
        } else {
            self.super_lvalue(lvalue, context);
        }
    }
    fn visit_terminator_kind(&mut self, block: BasicBlock, kind: &TerminatorKind<'a>) {
        match *kind {
            TerminatorKind::Call{..} => {
                debug!("found call!");
                self.found_call = true;
                return; // stop searching
            }
            _ => {}
        }
        self.super_terminator_kind(block, kind);
    }
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
    pub lists: HashMap<Lvalue<'a>, DefUseLists>,
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

fn get_one_optimization(mir: &Mir) -> Option<(BasicBlock, usize, BasicBlock, usize)> {
    let tduf = TempDefUseFinder::new(mir);
    if let Some(&temp) = tduf.get_temps_that_satisfy(mir).first() {
        if let Ok((use_bb, use_idx)) = tduf.get_one_use_as_idx(temp) {
            if let Ok((def_bb, def_idx)) = tduf.get_one_def_as_idx(temp) {
                return Some((use_bb, use_idx, def_bb, def_idx));
            }
        }
    }
    None
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
    fn add_to_map(&mut self, lvalue: &Lvalue<'a>) {
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
            AccessKind::Def => self.lists.entry(lvalue.clone())
                                        .or_insert(DefUseLists::new())
                                        .defs
                                        .push(ent),
            AccessKind::Use => self.lists.entry(lvalue.clone())
                                        .or_insert(DefUseLists::new())
                                        .uses
                                        .push(ent),
        };
    }
    fn get_one_use_as_idx(&self, temp: Temp) -> Result<(BasicBlock, usize), GetOneErr> {
        if self.lists[&Lvalue::Temp(temp)].uses.len() != 1 {
            return Err(GetOneErr::NotOne);
        };
        let use_loc = self.lists[&Lvalue::Temp(temp)].uses.first().unwrap();
        let use_bb = use_loc.basic_block;
        let use_idx = match use_loc.inner_location {
            InnerLocation::StatementIndex(idx) => idx,
            _ => return Err(GetOneErr::NotAStatement),
        };
        Ok((use_bb, use_idx))
    }
    fn get_one_def_as_idx(&self, temp: Temp) -> Result<(BasicBlock, usize), GetOneErr> {
        if self.lists[&Lvalue::Temp(temp)].defs.len() != 1 {
            return Err(GetOneErr::NotOne);
        };
        let def_loc = self.lists[&Lvalue::Temp(temp)].defs.first().unwrap();
        let def_bb = def_loc.basic_block;
        let def_idx = match def_loc.inner_location {
            InnerLocation::StatementIndex(idx) => idx,
            _ => return Err(GetOneErr::NotAStatement),
        };
        Ok((def_bb, def_idx))
    }
    fn get_temps_that_satisfy(&self, mir: &Mir<'a>) -> Vec<Temp> {
        self.lists.iter().filter(|&(lval, lists)| {
            if let &Lvalue::Temp(_) = lval {
                lists.uses.len() == 1 && lists.defs.len() == 1
            } else {
                false
            }
        }).map(|(lval, _)| {
            debug!("{:?} has 1 def and 1 use", lval);
            if let &Lvalue::Temp(tmp) = lval {
                (lval, tmp)
            } else {
                panic!("we already checked that it was a temp");
            }
        }).filter(|&(lval, tmp)| {
            if let Ok((use_bb, use_idx)) = self.get_one_use_as_idx(tmp) {
                // this checks the constraint: DEST is not borrowed (currently: not borrowed ever)
                let StatementKind::Assign(ref dest, ref rhs) = mir.basic_blocks()[use_bb]
                                                            .statements[use_idx]
                                                            .kind;

                // we can only really replace DEST = tmp
                // not more complex expressions
                if let &Rvalue::Use(Operand::Consume(ref use_lval)) = rhs {
                    if use_lval != lval {
                        return false; // we should never get here anyway
                    }
                } else {
                    return false;
                }
                if self.is_borrowed.contains(&dest) {
                    debug!("dest was borrowed: {:?}!", dest);
                    return false;
                }
                true
            } else {
                false
            }
        }).filter(|&(_, tmp)| {
            // 1) check all paths starting from Def(tmp = ...) to Use(DEST = tmp)
            //    * do not intermediately use DEST
            //    * and do not contain calls
            // 2) check all paths starting from Def(tmp = ...) to "exit"
            //    * either go through Use(DEST = tmp) or don't use DEST
            //    ** calls count as uses
            // 3) check that the address of DEST cannot change
            //    * currently, check that DEST is a plain (stack-allocated?) lvalue
            //      (not a projection)
            if let Ok((end_block, use_idx)) = self.get_one_use_as_idx(tmp) {
                if let Ok((start_block, def_idx)) = self.get_one_def_as_idx(tmp) {
                    let ref end_statement = mir.basic_blocks()[end_block].statements[use_idx];
                    let StatementKind::Assign(ref dest, _) = end_statement.kind;
                    if let &Lvalue::Projection(_) = dest {
                        return false;  // we don't replace projections for now
                    }
                    let ref start_block_data = mir.basic_blocks()[start_block];
                    return paths_satisfy_our_condition(dest,
                                                       end_statement,
                                                       end_block,
                                                       start_block,
                                                       def_idx,
                                                       start_block_data);
                }
            }
            false
        }).map(|(_, tmp)| {
            tmp
        }).collect()
    }
    // fn print(&self, mir: &Mir) {
    //     for (k, ref v) in self.lists.iter() {
    //         debug!("{:?} uses:", k);
    //         debug!("{:?}", v.uses);
    //         // this assertion was wrong
    //         // you can have an unused temporary, ex: the result of a call is never used
    //         //assert!(v.uses.len() > 0); // every temp should have at least one use
    //         v.uses.iter().map(|e| UseDefLocation::print(&e, mir)).count();
    //     }
    //     for (k, ref v) in self.lists.iter() {
    //         debug!("{:?} defs:", k);
    //         debug!("{:?}", v.defs);
    //         // this may be too strict? maybe the def was optimized out?
    //         //assert!(v.defs.len() > 0); // every temp should have at least one def
    //         v.defs.iter().map(|e| UseDefLocation::print(&e, mir)).count();
    //     }
    // }
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
        self.add_to_map(lvalue);
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
