// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
//
use rustc::mir::transform::{MirPass, MirSource, Pass};
use rustc::mir::repr::*;
use rustc::mir::visit::{Visitor, LvalueContext};
use rustc::ty::TyCtxt;
use std::collections::HashMap;

pub struct CacheTest;

impl<'tcx> MirPass<'tcx> for CacheTest {
    fn run_pass<'a>(&mut self, _tcx: TyCtxt<'a, 'tcx, 'tcx>, _src: MirSource, mir: &mut Mir<'tcx>) {
        debug!("predecessors: {:?}, successors: {:?}, dominators: {:?}", 
               mir.predecessors(), mir.successors(), mir.dominators());
        let d = TempDefFinder::new(mir);
        let u = TempUseFinder::new(mir);
        debug!("temp defs: {:?}", d.temp_defs);
        debug!("temp uses: {:?}", u.temp_uses);
    }
}

impl Pass for CacheTest {
    fn name(&self) -> &str { "cache_test" }
}

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct DefLoc {
    basic_block: BasicBlock,
    statement_index: usize,
}

#[derive(Debug, Hash, Eq, PartialEq)]
pub enum UseLoc {
    InStatement(StatementLoc),
    InTerminator(BasicBlock),
}

impl StatementLoc {
    fn dominates(&self, other: &Self, mir: &Mir) {
        if self.basic_block == other.basic_block {
            self.statement_index > other.statement_index;
        } else {
            mir.dominators().is_dominated_by(self.basic_block, other.basic_block);
        }
    }
}

//fn find_tmp_defs(mir: &Mir) -> HashMap<StatementLoc, Temp> {
//    let mut defs = HashMap::new();
//    for (bb_idx, data) in mir.basic_blocks().iter_enumerated() {
//        for (s_idx, s) in data.statements.iter().enumerate() {
//            match s.kind {
//                StatementKind::Assign(ref lvalue, _) => {
//                    match *lvalue {
//                        Lvalue::Temp(tmp_id) => {
//                            defs.insert(StatementLoc{basic_block: bb_idx, statement_index: s_idx}, tmp_id);
//                        },
//                        _ => { }
//                    }
//                }
//            }
//        }
//    }
//    defs
//}

fn find_tmp_uses(mir: &Mir) -> HashMap<StatementLoc, Temp> {
    let mut defs = HashMap::new();
    for (bb_idx, data) in mir.basic_blocks().iter_enumerated() {
        for (s_idx, s) in data.statements.iter().enumerate() {
            match s.kind {
                StatementKind::Assign(_, ref rvalue) => {
                    match *rvalue {
                        // TODO
                        _ => {}
                    }
                }
            }
        }
    }
    defs
}

struct TempUseFinder<'a> {
    temp_uses: HashMap<Temp, Vec<StatementLoc>>,
    mir: &'a Mir<'a>,
    curr_stmt_idx: usize,
    curr_block: BasicBlock,
}

impl<'a> TempUseFinder<'a> {
    fn new(mir: &'a Mir<'a>) -> Self {
        let mut tuf = TempUseFinder {
            temp_uses: HashMap::new(),
            mir: mir,
            curr_stmt_idx: 0,
            curr_block: START_BLOCK,
        };
        tuf.visit_mir(mir);
        tuf
    }
}

impl<'a> Visitor<'a> for TempUseFinder<'a> {
    fn visit_lvalue(&mut self,
                    lvalue: &Lvalue<'a>,
                    context: LvalueContext) {
        match context {
            LvalueContext::Store => { /* a store is a def, not a use*/ },
            //LvalueContext::Call => {},
            //LvalueContext::Drop => {},
            //LvalueContext::Inspect => {},
            //LvalueContext::Borrow { region: Region, kind: BorrowKind },
            //LvalueContext::Slice { from_start: usize, from_end: usize },
            //LvalueContext::Projection => {},
            //LvalueContext::Consume => {},
            _ => {
                match lvalue {
                    &Lvalue::Temp(tmp_id) => {
                        let l = StatementLoc {
                            statement_index: self.curr_stmt_idx,
                            basic_block: self.curr_block,
                        };
                        self.temp_uses.entry(tmp_id).or_insert(vec![]).push(l);
                    },
                    _ => {}
                }
            }
        }
    }
    fn visit_statement(&mut self, block: BasicBlock, statement: &Statement<'a>) {
        self.super_statement(block, statement);
        self.curr_stmt_idx += 1;
    }
    fn visit_basic_block_data(&mut self, block: BasicBlock, data: &BasicBlockData<'a>) {
        self.curr_stmt_idx = 0;
        self.curr_block = block;
        self.super_basic_block_data(block, data)
    }
}

struct TempDefFinder<'a> {
    pub temp_defs: HashMap<Temp, Vec<StatementLoc>>,
    mir: &'a Mir<'a>,
    curr_stmt_idx: usize,
}

impl<'a> TempDefFinder<'a> {
    fn new(mir: &'a Mir<'a>) -> Self {
        let mut tdf = TempDefFinder {
            temp_defs: HashMap::new(),
            mir: mir,
            curr_stmt_idx: 0,
        };
        tdf.visit_mir(mir);
        tdf
    }
}

impl<'a> Visitor<'a> for TempDefFinder<'a> {
    fn visit_basic_block_data(&mut self, block: BasicBlock, data: &BasicBlockData<'a>) {
        self.curr_stmt_idx = 0;
        self.super_basic_block_data(block, data)
    }

    fn visit_statement(&mut self, block: BasicBlock, statement: &Statement) {
        match statement.kind {
            StatementKind::Assign(ref lvalue, _) => {
                match *lvalue {
                    Lvalue::Temp(tmp_id) => {
                        let s_idx = self.curr_stmt_idx;
                        let l = StatementLoc{ 
                            basic_block: block, 
                            statement_index: s_idx,
                        };
                        self.temp_defs.entry(tmp_id).or_insert(vec![]).push(l);
                    },
                    _ => { }
                }
            }
        }
        self.curr_stmt_idx += 1;
    }
}

