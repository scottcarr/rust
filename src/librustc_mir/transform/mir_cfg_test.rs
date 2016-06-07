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
use mir_cfg::{MirCfg, Graph};
use pretty;

pub struct TestMirCfg;

impl TestMirCfg {
    pub fn new() -> TestMirCfg {
        TestMirCfg
    }
}

impl<'tcx> MirPass<'tcx> for TestMirCfg {
    fn run_pass<'a>(&mut self, tcx: TyCtxt<'a, 'tcx, 'tcx>, src: MirSource, mir: &mut Mir<'tcx>) {
        let s = MirCfg::new(mir);
        //let imm_mir = mir as & Mir<'tcx>;
        //for b in imm_mir.all_basic_blocks().iter() {
        debug!("num_nodes: {}", s.num_nodes());
        for &b in mir.all_basic_blocks().iter() {
            debug!("basic block: {:?} predecessors: {:?}", b, s.predecessors(b));
            //for p in s.predecessors(b) {
            //    debug!("{:?}", p);
            //}
            debug!("basic block: {:?} successors: {:?}", b, s.successors(b));
            //for su in s.successors(b) {
            //    debug!("{:?}", su);
            //}
        }
        pretty::dump_mir(tcx, "test super mir", &0, src, mir, None);
    }
}

impl Pass for TestMirCfg {}
