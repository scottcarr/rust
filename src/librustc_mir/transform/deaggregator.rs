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
use rustc_data_structures::indexed_vec::Idx;
use rustc::ty::{Ty, VariantKind, AdtDef};

pub struct Deaggregator;

impl Pass for Deaggregator {}

impl<'tcx> MirPass<'tcx> for Deaggregator {
    fn run_pass<'a>(&mut self, tcx: TyCtxt<'a, 'tcx, 'tcx>,
                    source: MirSource, mir: &mut Mir<'tcx>) {
        let node_id = source.item_id();
        let node_path = tcx.item_path_str(tcx.map.local_def_id(node_id));
        debug!("running on: {:?}", node_path);
        // we only run when mir_opt_level > 1
        match tcx.sess.opts.debugging_opts.mir_opt_level {
            Some(0) |
            Some(1) |
            None => { return; },
            _ => {}
        };
        if let MirSource::Fn(_) = source {} else { return; }

        for bb in mir.basic_blocks_mut() {
            while let Some(ai) = get_aggregate_statement(tcx, &bb.statements) {
                // do the replacement
                debug!("removing statement {:?}", ai.statement_index);
                let src_info = bb.statements[ai.statement_index].source_info;
                bb.statements.remove(ai.statement_index);
                for (i, (op, ty)) in ai.operands.iter().zip(ai.types.iter()).enumerate() {
                    let rhs = Rvalue::Use(op.clone());
                    let lhs_cast = if ai.adt_def.variants.len() > 1 {
                        Lvalue::Projection(Box::new(LvalueProjection {
                            base: ai.lhs.clone(),
                            elem: ProjectionElem::Downcast(ai.adt_def, ai.variant), 
                        }))
                    } else {
                        ai.lhs.clone()
                    };
                    let lhs_proj = Lvalue::Projection(Box::new(LvalueProjection {
                        base: lhs_cast,
                        elem: ProjectionElem::Field(Field::new(i), ty),
                    }));
                    let new_statement = Statement { 
                        source_info: src_info, 
                        kind: StatementKind::Assign(lhs_proj, rhs),
                    };
                    debug!("inserting: {:?} @ {:?}", new_statement, ai.statement_index + i);
                    bb.statements.insert(ai.statement_index + i, new_statement);
                }
            }
        }
    }
}

fn get_aggregate_statement<'a, 'tcx, 'b>(tcx: TyCtxt<'a, 'tcx, 'tcx>,
                                         statements: &Vec<Statement<'tcx>>)
                                         -> Option<AggInfo<'tcx>> {
    for (i, statement) in statements.iter().enumerate() {
        //debug!("looking at stmt: {:?}", statement);
        let StatementKind::Assign(ref lhs, ref rhs) = statement.kind;
        if let &Rvalue::Aggregate(ref kind, ref operands) = rhs {
            if let &AggregateKind::Adt(adt_def, variant, substs) = kind {
                if operands.len() > 0 { // don't deaggregate ()
                    //if adt_def.variants.len() == 1 { // only deaggrate structs for now
                    debug!("getting variant {:?}", variant);
                    debug!("for adt_def {:?}", adt_def);
                    let variant_def = &adt_def.variants[variant];
                    if variant_def.kind == VariantKind::Struct {
                        let types: Vec<_> = variant_def.fields.iter().map(|f| {
                            f.ty(tcx, substs)
                        }).collect();
                        debug!("found a aggregate: {:?}", statement);
                        debug!("ops: {:?}", operands);
                        return Some( AggInfo {
                            statement_index: i,
                            lhs: lhs.clone(),
                            operands: operands.clone(),
                            types: types,
                            adt_def: adt_def,
                            variant: variant,
                        });
                    }
                    //}
                }
            }
        } 
    };
    None
}

struct AggInfo<'tcx> {
    statement_index: usize,
    lhs: Lvalue<'tcx>,
    operands: Vec<Operand<'tcx>>,
    types: Vec<Ty<'tcx>>,
    adt_def: AdtDef<'tcx>,
    variant: usize,
}