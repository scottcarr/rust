// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::{iter, slice};
use rustc_data_structures::indexed_vec::IndexVec;
use rustc_data_structures::graph_algorithms::{Graph, GraphPredecessors, GraphSuccessors};
use mir::repr::{Mir, BasicBlock, START_BLOCK};
use mir::cache::Cache;

impl MirCfg {
    pub fn new<'a, 'tcx>(mir: &Mir, cache: &Cache) -> Self {
        MirCfg {
            predecessors: cache.predecessors(mir).clone(),
            successors: calculate_successors(mir),
            n_nodes: mir.basic_blocks().len(),
            start_node: START_BLOCK,
        }
    }
}

#[derive(Clone, Debug)]
pub struct MirCfg {
    predecessors: IndexVec<BasicBlock,Vec<BasicBlock>>,
    successors: IndexVec<BasicBlock,Vec<BasicBlock>>,
    start_node: BasicBlock,
    n_nodes: usize,
}

impl Graph for MirCfg {

    type Node = BasicBlock;

    fn num_nodes(&self) -> usize { self.n_nodes }

    fn start_node(&self) -> Self::Node { self.start_node }

    fn predecessors<'graph>(&'graph self, node: Self::Node)
                            -> <Self as GraphPredecessors<'graph>>::Iter
    {
        self.predecessors[node].iter().cloned()
    }
    fn successors<'graph>(&'graph self, node: Self::Node)
                            -> <Self as GraphSuccessors<'graph>>::Iter
    {
        self.successors[node].iter().cloned()
    }
}

impl<'g> GraphPredecessors<'g> for MirCfg {
    type Item = BasicBlock;
    type Iter = iter::Cloned<slice::Iter<'g, BasicBlock>>;
}

impl<'g>  GraphSuccessors<'g> for MirCfg {
   type Item = BasicBlock;
    type Iter = iter::Cloned<slice::Iter<'g, BasicBlock>>;
}

fn calculate_successors<'a, 'tcx>(mir: &'a Mir<'tcx>) -> IndexVec<BasicBlock, Vec<BasicBlock>> {
    let mut successors = IndexVec::from_elem(vec![], mir.basic_blocks());
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
