// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use rustc::middle::const_val::ConstVal;
use rustc::ty::{Ty, TyCtxt, FnOutput, ClosureSubsts};
use rustc::hir::def_id::DefId;
use rustc::mir::repr::*;
use rustc::mir::visit::{MutVisitor, LvalueContext};
use rustc::mir::transform::{MirPass, MirSource, Pass};
use syntax::codemap::Span;
use rustc::ty::subst::Substs;
use rustc_const_math::ConstUsize;
use pretty;

struct EliminateMovesVisitor<'a, 'tcx: 'a> {
    tcx: TyCtxt<'a, 'tcx, 'tcx>,
}

impl<'a, 'tcx> EliminateMovesVisitor<'a, 'tcx> {
    pub fn new(tcx: TyCtxt<'a, 'tcx, 'tcx>) -> Self {
        EliminateMovesVisitor {
            tcx: tcx
        }
    }
}

impl<'a, 'tcx> MutVisitor<'tcx> for EliminateMovesVisitor<'a, 'tcx> {
    fn visit_mir(&mut self, mir: &mut Mir<'tcx>) {
        self.super_mir(mir);
    }

    fn visit_basic_block_data(&mut self,
                              block: BasicBlock,
                              data: &mut BasicBlockData<'tcx>) {
        debug!("visited basic_block_data: {:?}", data);
        self.super_basic_block_data(block, data);
    }

    fn visit_scope_data(&mut self,
                        scope_data: &mut ScopeData) {
        debug!("visited scope_data: {:?}", scope_data);
        self.super_scope_data(scope_data);
    }

    fn visit_statement(&mut self,
                       block: BasicBlock,
                       statement: &mut Statement<'tcx>) {
        debug!("visited statement: {:?}", statement);
        self.super_statement(block, statement);
    }

    fn visit_assign(&mut self,
                    block: BasicBlock,
                    lvalue: &mut Lvalue<'tcx>,
                    rvalue: &mut Rvalue<'tcx>) {
        debug!("visited assign. lvalue: {:?}, rvalue: {:?}", lvalue, rvalue);
        self.super_assign(block, lvalue, rvalue);
    }

    fn visit_terminator(&mut self,
                        block: BasicBlock,
                        terminator: &mut Terminator<'tcx>) {
        debug!("visited terminator: {:?}", terminator);
        self.super_terminator(block, terminator);
    }

    fn visit_terminator_kind(&mut self,
                             block: BasicBlock,
                             kind: &mut TerminatorKind<'tcx>) {
        debug!("visited terminator_kind: {:?}", kind);
        self.super_terminator_kind(block, kind);
    }

    fn visit_rvalue(&mut self,
                    rvalue: &mut Rvalue<'tcx>) {
        debug!("visited rvalue: {:?}", rvalue);
        self.super_rvalue(rvalue);
    }

    fn visit_operand(&mut self,
                     operand: &mut Operand<'tcx>) {
        debug!("visited operand: {:?}", operand);
        self.super_operand(operand);
    }

    fn visit_lvalue(&mut self,
                    lvalue: &mut Lvalue<'tcx>,
                    context: LvalueContext) {
        debug!("visited lvalue: {:?}, context: {:?}", lvalue, context);
        self.super_lvalue(lvalue, context);
    }

    fn visit_projection(&mut self,
                        lvalue: &mut LvalueProjection<'tcx>,
                        context: LvalueContext) {
        debug!("visited projection: {:?}, context: {:?}", lvalue, context);
        self.super_projection(lvalue, context);
    }

    fn visit_projection_elem(&mut self,
                             lvalue: &mut LvalueElem<'tcx>,
                             context: LvalueContext) {
        debug!("visited projection_elem: {:?}, context: {:?}", lvalue, context);
        self.super_projection_elem(lvalue, context);
    }

    fn visit_branch(&mut self,
                    source: BasicBlock,
                    target: BasicBlock) {
        debug!("visited branch. source: {:?}, target: {:?}", source, target);
        self.super_branch(source, target);
    }

    fn visit_constant(&mut self,
                      constant: &mut Constant<'tcx>) {
        debug!("visited constant: {:?}", constant);
        self.super_constant(constant);
    }

    fn visit_literal(&mut self,
                     literal: &mut Literal<'tcx>) {
        debug!("visited literal: {:?}", literal);
        self.super_literal(literal);
    }

    fn visit_def_id(&mut self,
                    def_id: &mut DefId) {
        debug!("visited def_id: {:?}", def_id);
        self.super_def_id(def_id);
    }

    fn visit_span(&mut self,
                  span: &mut Span) {
        debug!("visited span: {:?}", span);
        self.super_span(span);
    }

    fn visit_fn_output(&mut self,
                       fn_output: &mut FnOutput<'tcx>) {
        debug!("visited fn_output: {:?}", fn_output);
        self.super_fn_output(fn_output);
    }

    fn visit_ty(&mut self,
                ty: &mut Ty<'tcx>) {
        debug!("visited ty: {:?}", ty);
        self.super_ty(ty);
    }

    fn visit_substs(&mut self,
                    substs: &mut &'tcx Substs<'tcx>) {
        debug!("visited substs: {:?}", substs);
        self.super_substs(substs);
    }

    fn visit_closure_substs(&mut self,
                            substs: &mut ClosureSubsts<'tcx>) {
        debug!("visited closure_substs: {:?}", substs);
        self.super_closure_substs(substs);
    }

    fn visit_const_val(&mut self,
                       const_val: &mut ConstVal) {
        debug!("visited const_val: {:?}", const_val);
        self.super_const_val(const_val);
    }

    fn visit_const_usize(&mut self,
                         const_usize: &mut ConstUsize) {
        debug!("visited const_usize: {:?}", const_usize);
        self.super_const_usize(const_usize);
    }

    fn visit_typed_const_val(&mut self,
                             val: &mut TypedConstVal<'tcx>) {
        debug!("visited typed_const_val: {:?}", val);
        self.super_typed_const_val(val);
    }

    fn visit_var_decl(&mut self,
                      var_decl: &mut VarDecl<'tcx>) {
        debug!("visited var_decl: {:?}", var_decl);
        self.super_var_decl(var_decl);
    }

    fn visit_temp_decl(&mut self,
                       temp_decl: &mut TempDecl<'tcx>) {
        debug!("visited temp_decl: {:?}", temp_decl);
        self.super_temp_decl(temp_decl);
    }

    fn visit_arg_decl(&mut self,
                      arg_decl: &mut ArgDecl<'tcx>) {
        debug!("visited arg_decl: {:?}", arg_decl);
        self.super_arg_decl(arg_decl);
    }

    fn visit_scope_id(&mut self,
                      scope_id: &mut ScopeId) {
        debug!("visited scope_id: {:?}", scope_id);
        self.super_scope_id(scope_id);
    }
}

pub struct EliminateMoves;

impl Pass for EliminateMoves {}

impl<'tcx> MirPass<'tcx> for EliminateMoves {
    fn run_pass<'a>(&mut self, tcx: TyCtxt<'a, 'tcx, 'tcx>,
                    src: MirSource, mir: &mut Mir<'tcx>) {
        EliminateMovesVisitor::new(tcx).visit_mir(mir);
        pretty::dump_mir(tcx, "eliminate_moves", &0, src, mir, None);
    }
}
