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
use rustc_data_structures::indexed_vec::{Idx, IndexVec};

struct MoveUpPropagation;

impl<'tcx> MirPass<'tcx> for MoveUpPropagation {
    fn run_pass<'a>(&mut self, _tcx: TyCtxt<'a, 'tcx, 'tcx>, _src: MirSource, mir: &mut Mir<'tcx>) {
        for bb in mir.basic_blocks_mut() {
            for (i, stmt) in bb.statements().iter().enumerate() {
                if let Statement(_, 
                    StatementKind(Assign(lval, TempDecl(tmpId)))) = stmt {
                    try_optimze(i, bb);
                }
            }
        }
    }
}

fn try_optimze(stmt_idx: usize, bb: BasicBlock) {
}

fn walk_backwards(stmt_idx: usize, bb: BasicBlock) {
    if stmt_idx > 0 {
        stmt_idx -= 1
    } else {

    }
}

impl Pass for MoveUpPropagation {}

// one potential problem with only looking at
// statements is what if the temporary I'm eliminating
// is used in a terminator?