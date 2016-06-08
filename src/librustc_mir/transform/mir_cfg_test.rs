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
//use rustc_data_structures::graph_algorithms::iterate::pre_order_walk;
use std::collections::HashMap;
use mir_cfg::MirCfg;
use mir_cfg::ReachingDefinitions;
use pretty;

pub struct TestMirCfg;

impl TestMirCfg {
    pub fn new() -> Self {
        TestMirCfg
    }
}

impl<'tcx> MirPass<'tcx> for TestMirCfg {
    fn run_pass<'a>(&mut self, tcx: TyCtxt<'a, 'tcx, 'tcx>, src: MirSource, mir: &mut Mir<'tcx>) {
        //let s = MirCfg::new(mir);
        let r = ReachingDefinitions::new(mir);
        //let d = dominators(&r.mir_cfg);
        debug!("num_nodes: {}", r.mir_cfg.num_nodes());
        for &b in mir.all_basic_blocks().iter() {
            debug!("basic block: {:?} predecessors: {:?} successors: {:?}, immediate_dominator: {:?}", 
                   b, r.mir_cfg.predecessors(b), r.mir_cfg.successors(b), d.immediate_dominator(b));
        }
        debug!("reaching defs for temps: {:?}", r.tmp_defs);
        pretty::dump_mir(tcx, "test_mir_cfg", &0, src, mir, None);
    }
}


impl Pass for TestMirCfg {}
