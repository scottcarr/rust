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
use rustc::ty::TyCtxt;

pub struct CacheTest;

impl<'tcx> MirPass<'tcx> for CacheTest {
    fn run_pass<'a>(&mut self, _tcx: TyCtxt<'a, 'tcx, 'tcx>, _src: MirSource, mir: &mut Mir<'tcx>) {
        debug!("predecessors: {:?}, successors: {:?}, dominators: {:?}", 
               mir.predecessors(), mir.successors(), mir.dominators());
    }
}

impl Pass for CacheTest {
    fn name(&self) -> &str { "cache_test" }
}
