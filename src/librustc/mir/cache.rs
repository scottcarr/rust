// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::cell::{Ref, RefCell};
use rustc_data_structures::indexed_vec::IdxVec;
use rustc_data_structures::graph_algorithms::dominators::{dominators,Dominators};
use mir::repr::{Mir, BasicBlock};
use mir::mir_cfg::MirCfg;

use rustc_serialize as serialize;

#[derive(Clone)]
pub struct Cache {
    predecessors: RefCell<Option<IdxVec<BasicBlock, Vec<BasicBlock>>>>,
    successors: RefCell<Option<IdxVec<BasicBlock, Vec<BasicBlock>>>>,
    dominators: RefCell<Option<Dominators<MirCfg>>>,
}

impl serialize::Encodable for Cache {
    fn encode<S: serialize::Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        serialize::Encodable::encode(&(), s)
    }
}

impl serialize::Decodable for Cache {
    fn decode<D: serialize::Decoder>(d: &mut D) -> Result<Self, D::Error> {
        serialize::Decodable::decode(d).map(|_v: ()| Self::new())
    }
}

impl Cache {
    pub fn new() -> Self {
        Cache {
            predecessors: RefCell::new(None),
            successors: RefCell::new(None),
            dominators: RefCell::new(None),
        }
    }

    pub fn invalidate(&self) {
        // FIXME: consider being more fine-grained
        *self.predecessors.borrow_mut() = None;
        *self.successors.borrow_mut() = None;
        *self.dominators.borrow_mut() = None;
    }

    pub fn predecessors(&self, mir: &Mir) -> Ref<IdxVec<BasicBlock, Vec<BasicBlock>>> {
        if self.predecessors.borrow().is_none() {
            *self.predecessors.borrow_mut() = Some(calculate_predecessors(mir));
        }

        Ref::map(self.predecessors.borrow(), |p| p.as_ref().unwrap())
    }

    pub fn successors(&self, mir: &Mir) -> Ref<IdxVec<BasicBlock, Vec<BasicBlock>>> {
        if self.successors.borrow().is_none() {
            *self.successors.borrow_mut() = Some(calculate_successors(mir));
        }

        Ref::map(self.successors.borrow(), |p| p.as_ref().unwrap())
    }

    pub fn dominators(&self, mir: &Mir) -> Ref<Dominators<MirCfg>> {
        if self.dominators.borrow().is_none() {
            *self.dominators.borrow_mut() = Some(calculate_dominators(mir, self));
        }

        Ref::map(self.dominators.borrow(), |p| p.as_ref().unwrap())
    }
}

fn calculate_predecessors(mir: &Mir) -> IdxVec<BasicBlock, Vec<BasicBlock>> {
    let mut result = IdxVec::from_elem(vec![], mir.basic_blocks());
    for (bb, data) in mir.basic_blocks().iter_enumerated() {
        if let Some(ref term) = data.terminator {
            for &tgt in term.successors().iter() {
                result[tgt].push(bb);
            }
        }
    }
    for ps in result.iter_mut() {
        ps.sort();
        ps.dedup();
    }
    result
}

fn calculate_successors<'a, 'tcx>(mir: &'a Mir<'tcx>) -> IdxVec<BasicBlock, Vec<BasicBlock>> {
    let mut successors = IdxVec::from_elem(vec![], mir.basic_blocks());
    for (bb, data) in mir.basic_blocks().iter_enumerated() {
        if let Some(ref term) = data.terminator {
            successors[bb].append(term.successors().to_mut());
        }
    }
    for ss in successors.iter_mut() {
        ss.sort();
        ss.dedup();
    }
    successors
}

fn calculate_dominators(mir: &Mir, cache: &Cache) -> Dominators<MirCfg> {
    let m = MirCfg::new(mir, cache);
    dominators(&m)
}

