// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
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
use rustc_data_structures::graph_algorithms::dominators::dominators;
use rustc_data_structures::graph_algorithms::iterate::pre_order_walk;
use rustc_data_structures::graph_algorithms::Graph;
use std::collections::HashMap;
use mir_cfg::MirCfg;
use pretty;

pub struct TestMirCfg;

impl TestMirCfg {
    pub fn new() -> Self {
        TestMirCfg
    }
}

impl<'tcx> MirPass<'tcx> for TestMirCfg {
    fn run_pass<'a>(&mut self, tcx: TyCtxt<'a, 'tcx, 'tcx>, src: MirSource, mir: &mut Mir<'tcx>) {
        let s = MirCfg::new(mir);
        let d = dominators(&s);
        debug!("num_nodes: {}", s.num_nodes());
        for &b in mir.all_basic_blocks().iter() {
            debug!("basic block: {:?} predecessors: {:?} successors: {:?}, immediate_dominator: {:?}", 
                   b, s.predecessors(b), s.successors(b), d.immediate_dominator(b));
        }
        let r = ReachingDefinitions::new(&s, mir);
        pretty::dump_mir(tcx, "test_mir_cfg", &0, src, mir, None);
    }
}

pub struct ReachingDefinitions<'a> {
    defs: HashMap<&'a Statement<'tcx>, &'a Lvalue<'tcx>>,
}

fn find_defs<'tcx, 'a>(mir: &'a mut Mir<'tcx>) -> HashMap<&'a Statement<'tcx>, &'a Lvalue<'tcx>> {
    let mut defs = HashMap::new();
    for data in mir.all_basic_blocks().iter().map(|&b| mir.basic_block_data(b)) {
        for s in data.statements.iter() {
            match s.kind {
                StatementKind::Assign(ref lvalue, _) => {
                    defs.insert(s, lvalue);
                }
            }
        }
    }
    defs
}

impl<'a> ReachingDefinitions<'a> {
    pub fn new<'tcx>(mir_cfg: &'a MirCfg, mir: &'a mut Mir<'tcx>) -> Self {
        let r = ReachingDefinitions {
            defs: find_defs(mir),
        };
        r
    }
}

impl Pass for TestMirCfg {}
