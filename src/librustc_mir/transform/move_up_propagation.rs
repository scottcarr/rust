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
use rustc::mir::visit::{Visitor, LvalueContext, MutVisitor};
use std::collections::HashSet;
use rustc_data_structures::tuple_slice::TupleSlice;
use rustc_data_structures::bitvec::BitVector;
use rustc_data_structures::indexed_vec::{IndexVec, Idx};

pub struct MoveUpPropagation;

impl Pass for MoveUpPropagation {}

impl<'tcx> MirPass<'tcx> for MoveUpPropagation {
    fn run_pass<'a>(&mut self,
                    tcx: TyCtxt<'a, 'tcx, 'tcx>,
                    src: MirSource,
                    mir: &mut Mir<'tcx>) {
        // we only return when mir_opt_level > 1
        match tcx.sess.opts.debugging_opts.mir_opt_level {
            Some(0) |
            Some(1) |
            None => { return; },
            _ => {}
        };
        let node_id = src.item_id();
        let node_path = tcx.item_path_str(tcx.map.local_def_id(node_id));
        debug!("move-up-propagation on {:?}", node_path);

        let mut opt_counter = 0;
        let mut dead_temps = BitVector::new(mir.temp_decls.len());
        let mut dead_vars = BitVector::new(mir.var_decls.len());
        while let Some((tmp, use_bb, use_idx, def_bb, def_idx)) = get_one_optimization(mir) {
            let new_statement_kind = get_replacement_statement(mir,
                                                               use_bb,
                                                               use_idx,
                                                               def_bb,
                                                               def_idx);
            {
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
            }

            // tmp is really a local index (an index into an imaginary vector...)
            // so we need to convert it back to a Temp
            if let Some(idx) = mir.from_local_index_to_temp(tmp) {
                debug!("deleting the decl of: {:?}", idx);
                dead_temps.insert(idx.index());
            }
            if let Some(idx) = mir.from_local_index_to_var(tmp) {
                debug!("deleting the decl of: {:?}", idx);
                dead_vars.insert(idx.index());
            }

            opt_counter += 1;
        }
        cleanup_temps(mir, dead_temps);
        cleanup_vars(mir, dead_vars);
        debug!("we did {:?} optimizations", opt_counter);
    }
}

fn cleanup_temps(mir: &mut Mir, dead_temps: BitVector) {
    let mut curr = 0;
    let mut new_vals: IndexVec<Temp, Temp> = IndexVec::with_capacity(mir.temp_decls.len());
    for i in 0..mir.temp_decls.len() {
        if dead_temps.contains(i) {
            // if the value is dead, we don't have any mapping from old to new
            new_vals.push(Temp::new(0)); // we should never encounter these anyway
        } else {
            new_vals.push(Temp::new(curr));
            curr += 1;
        }
    }
    let mut tr = TempRewriter { new_vals: new_vals };
    tr.visit_mir(mir);
    let mut new_decls = IndexVec::new();
    for (idx, e) in mir.temp_decls.iter_enumerated() {
        if !dead_temps.contains(idx.index()) {
            new_decls.push(e.clone());
        }
    }
    mir.temp_decls = new_decls;
}

struct TempRewriter {
    new_vals: IndexVec<Temp, Temp>,
}

impl<'a> MutVisitor<'a> for TempRewriter {
    fn visit_lvalue(&mut self, lvalue: &mut Lvalue<'a>, context: LvalueContext) {
        match lvalue {
            &mut Lvalue::Temp(idx) => {
                *lvalue = Lvalue::Temp(self.new_vals[idx]);
            }
            _ => {}
        }
        self.super_lvalue(lvalue, context);
    }
}

fn cleanup_vars(mir: &mut Mir, dead_vars: BitVector) {
    let mut curr = 0;
    let mut new_vals: IndexVec<Var, Var> = IndexVec::with_capacity(mir.var_decls.len());
    for i in 0..mir.var_decls.len() {
        if dead_vars.contains(i) {
            // if the value is dead, we don't have any mapping from old to new
            new_vals.push(Var::new(0)); // we should never encounter these anyway
        } else {
            new_vals.push(Var::new(curr));
            curr += 1;
        }
    }
    let mut tr = VarRewriter { new_vals: new_vals };
    tr.visit_mir(mir);
    let mut new_decls = IndexVec::new();
    for (idx, e) in mir.var_decls.iter_enumerated() {
        if !dead_vars.contains(idx.index()) {
            new_decls.push(e.clone());
        }
    }
    mir.var_decls = new_decls;
}

struct VarRewriter {
    new_vals: IndexVec<Var, Var>,
}

impl<'a> MutVisitor<'a> for VarRewriter {
    fn visit_lvalue(&mut self, lvalue: &mut Lvalue<'a>, context: LvalueContext) {
        match lvalue {
            &mut Lvalue::Var(idx) => {
                *lvalue = Lvalue::Var(self.new_vals[idx]);
            }
            _ => {}
        }
        self.super_lvalue(lvalue, context);
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash)]
struct UseDefLocation {
    basic_block: BasicBlock,
    inner_location: InnerLocation,
}

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

#[derive(Clone)]
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
    pub lists: IndexVec<Local, DefUseLists>,
    pub is_borrowed: HashSet<Lvalue<'a>>,
    curr_basic_block: BasicBlock,
    statement_index: usize,
    kind: AccessKind,
    is_in_terminator: bool,
    mir: &'a Mir<'a>,
}

enum AccessKind {
    Def,
    Use,
}

fn get_one_optimization<'a>(mir: &Mir<'a>)
                            -> Option<(Local, BasicBlock, usize, BasicBlock, usize)> {
    let tduf = TempDefUseFinder::new(mir);
    match tduf.get_temp_that_satisfies(mir) {
        Some(local) => {
            let (def_bb, def_idx) = tduf.get_one_def_as_idx(local).unwrap();
            let (use_bb, use_idx) = tduf.get_one_use_as_idx(local).unwrap();
            Some((local, use_bb, use_idx, def_bb, def_idx))
        },
        _ => None
    }
}

impl<'a> TempDefUseFinder<'a> {
    fn new(mir: &'a Mir<'a>) -> Self {
        let local_vars_len = mir.arg_decls.len() + mir.temp_decls.len() + mir.var_decls.len();
        let mut tuc = TempDefUseFinder {
            lists: IndexVec::from_elem_n(DefUseLists::new(), local_vars_len),
            is_borrowed: HashSet::new(),
            curr_basic_block: START_BLOCK,
            statement_index: 0,
            kind: AccessKind::Def, // will get updated when we see an assign
            is_in_terminator: false,
            mir: mir,
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
        // check if lvalue is actually a local
        match lvalue {
            &Lvalue::Var(_) |
            &Lvalue::Arg(_) |
            &Lvalue::Temp(_) => {
                let idx = self.mir.local_index(lvalue).unwrap();
                match self.kind {
                    AccessKind::Def => {
                        self.lists[idx].defs.push(ent)
                    }
                    AccessKind::Use => {
                        self.lists[idx].uses.push(ent)
                    }
                };
            }
            _ => {}
        }
    }

    fn get_one_use_as_idx(&self, temp: Local) -> Option<(BasicBlock, usize)> {
        if self.lists[temp].uses.len() != 1 {
            return None;
        };
        let use_loc = self.lists[temp].uses.first().unwrap();
        let use_bb = use_loc.basic_block;
        let use_idx = match use_loc.inner_location {
            InnerLocation::StatementIndex(idx) => idx,
            _ => { return None; },
        };
        Some((use_bb, use_idx))
    }
    fn get_one_def_as_idx(&self, temp: Local) -> Option<(BasicBlock, usize)> {
        if self.lists[temp].defs.len() != 1 {
            return None;
        };
        let def_loc = self.lists[temp].defs.first().unwrap();
        let def_bb = def_loc.basic_block;
        let def_idx = match def_loc.inner_location {
            InnerLocation::StatementIndex(idx) => idx,
            _ => { return None; },
        };
        Some((def_bb, def_idx))
    }

    fn has_complicated_rhs(&self, use_bb: BasicBlock, use_idx: usize, mir: &Mir<'a>) -> bool {
        let StatementKind::Assign(_, ref rhs) = mir.basic_blocks()[use_bb]
                                                    .statements[use_idx]
                                                    .kind;
        // we can only really replace DEST = tmp
        // not more complex expressions
        if let &Rvalue::Use(Operand::Consume(ref use_rval)) = rhs {
            if let Some(_) = mir.local_index(use_rval) {
                return false;
            }
        }
        return true;
    }
    fn is_dest_borrowed(&self, use_bb: BasicBlock, use_idx: usize, mir: &Mir<'a>) -> bool {
        // this checks the constraint: DEST is not borrowed (currently: not borrowed ever)
        let StatementKind::Assign(ref dest, _) = mir.basic_blocks()[use_bb]
                                                    .statements[use_idx]
                                                    .kind;
        self.is_borrowed.contains(&dest)
    }

    fn paths_satisfy(&self,
                     end_block: BasicBlock,
                     use_idx: usize,
                     start_block: BasicBlock,
                     def_idx: usize,
                     mir: &Mir<'a>)
                     -> bool {
        // 1) check all paths starting from Def(tmp = ...) to Use(DEST = tmp)
        //    * do not intermediately use DEST
        //    * and do not contain calls
        // 2) check all paths starting from Def(tmp = ...) to "exit"
        //    * either go through Use(DEST = tmp) or don't use DEST
        //    ** calls count as uses
        // 3) check that the address of DEST cannot change
        //    * currently, check that DEST is a plain (stack-allocated?) lvalue
        //      (not a projection)
        let ref end_statement = mir.basic_blocks()[end_block].statements[use_idx];
        let StatementKind::Assign(ref dest, _) = end_statement.kind;
        if let &Lvalue::Projection(_) = dest {
            return false;  // we don't replace projections for now
        }
        let ref start_block_data = mir.basic_blocks()[start_block];
        let mut upf = UseOnPathFinder {
            dest: dest,
            end_statement: end_statement,
            end_block: end_block,
            found_intermediate_use_of_dest: false,
            found_call: false,
            found_end_statement: false,
        };
        upf.visit_first_block(start_block, start_block_data, def_idx);
        upf.found_end_statement && !upf.found_intermediate_use_of_dest && !upf.found_call
    }

    fn get_temp_that_satisfies(&self, mir: &Mir<'a>) -> Option<Local> {
        let result = self.lists.iter_enumerated().find(|&(local,_)| {
            if mir.local_index(&Lvalue::ReturnPointer).unwrap() == local {
                debug!("local was return pointer!");
                return false;
            }
            let (use_bb, use_idx) = match self.get_one_use_as_idx(local) {
                Some(x) => x,
                None => {
                    debug!("local did not have one use!");
                    return false;
                }
            };
            let (def_bb, def_idx) = match self.get_one_def_as_idx(local) {
                Some(x) => x,
                None => {
                    debug!("local did not have one def!");
                    return false;
                }
            };
            if self.is_dest_borrowed(use_bb, use_idx, mir) {
                debug!("DEST was borrowed!");
                return false;
            }
            if self.has_complicated_rhs(use_bb, use_idx, mir) {
                debug!("the rhs of DEST = rhs was complicated!");
                return false;
            }
            if !self.paths_satisfy(use_bb, use_idx, def_bb, def_idx, mir) {
                return false;
            }
            return true;
        });
        match result {
            Some((local, _)) => Some(local),
            _ => None,
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
