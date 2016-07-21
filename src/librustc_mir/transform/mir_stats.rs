// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! This pass collects and dumps some stats about a Mir

use rustc::ty::TyCtxt;
use rustc::mir::repr::*;
use rustc::mir::transform::{Pass, MirPassHook, MirSource};
use transform::dump_mir::Disambiguator;
//use rustc::mir::visit::Visitor;
//use pretty;

pub struct MirStats;

impl<'tcx> MirPassHook<'tcx> for MirStats {
    fn on_mir_pass<'a>(
        &mut self,
        tcx: TyCtxt<'a, 'tcx, 'tcx>,
        src: MirSource,
        mir: &Mir<'tcx>,
        pass: &Pass,
        is_after: bool)
    {
        if tcx.sess.opts.debugging_opts.print_mir_stats {
            let node_id = src.item_id();
            let node_path = tcx.item_path_str(tcx.map.local_def_id(node_id));
            let disambiguator = Disambiguator::new(pass, is_after);
            let num_stmts = mir.basic_blocks().iter().fold(0usize, |total, bb| {
                total + bb.statements.len()
            });
            println!("{}-{} on {}. num temps: {}. num basic_blocks: {}. num statements: {}.",
                     pass.name(),
                     disambiguator,
                     node_path,
                     mir.temp_decls.len(),
                     mir.basic_blocks().len(),
                     num_stmts);
        }
    }
}

impl Pass for MirStats {}

// struct MirStatsCollector {
//     num_temporary_vars: usize,
// }

// impl<'tcx> Visitor<'tcx> for MirStatsCollector<'tcx> {
//     fn visit_lvalue(&mut self,
//                     lvalue: &Lvalue<'tcx>,
//                     context: LvalueContext) {
//         if let Lvalue::Temp(_) = lvalue {
//             num_tempory_vars += 1;
//         }
//         self.super_lvalue(lvalue, context);
//     }
// }