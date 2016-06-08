// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use rustc::mir::repr::*;
use std::fmt::Debug;
use std::hash::Hash;
use std::iter;
use std::slice;
use std::convert::From;
use std::collections::HashMap;

use traversal;
use rustc_data_structures::graph_algorithms::{Graph, GraphPredecessors, GraphSuccessors, NodeIndex};

pub type NodeType = BasicBlock;

fn compute_predecessors<'a, 'tcx>(mir: &'a Mir<'tcx>) -> HashMap<NodeType, Vec<NodeType>> {
    let mut predecessors = HashMap::new();
    predecessors.insert(START_BLOCK, vec![]);
    for (from, data) in traversal::preorder(mir) {
        if let Some(ref term) = data.terminator {
            for &tgt in term.successors().iter() {
                predecessors.entry(tgt).or_insert(vec![]).push(from);
            }
        }
    }
    for ps in predecessors.values_mut() {
        ps.sort();
        ps.dedup();
    }
    predecessors
}

fn compute_successors<'a, 'tcx>(mir: &'a Mir<'tcx>) -> HashMap<NodeType, Vec<NodeType>> {
    let mut successors = HashMap::new();
    for (from, data) in traversal::preorder(mir) {
        if let Some(ref term) = data.terminator {
            successors.entry(from).or_insert(vec![]).append(term.successors().to_mut());
        }
    }
    for ss in successors.values_mut() {
        ss.sort();
        ss.dedup();
    }
    successors
}

impl MirCfg {
    pub fn new<'a, 'tcx>(mir: &'a Mir<'tcx>) -> Self {
        MirCfg { 
            predecessors: compute_predecessors(mir), 
            successors: compute_successors(mir), 
            n_nodes: mir.basic_blocks.len(), 
            start_node: START_BLOCK,
        }
    }
}

pub struct MirCfg {
    predecessors: HashMap<NodeType,Vec<NodeType>>,
    successors: HashMap<NodeType,Vec<NodeType>>,
    start_node: NodeType,
    n_nodes: usize,
}

impl Graph for MirCfg {

    type Node = NodeType;

    fn num_nodes(&self) -> usize { self.n_nodes }

    fn start_node(&self) -> Self::Node { self.start_node }

    fn predecessors<'graph>(&'graph self, node: Self::Node)
                            -> <Self as GraphPredecessors<'graph>>::Iter
    {
        self.predecessors[&node].iter().cloned()
    }
    fn successors<'graph>(&'graph self, node: Self::Node)
                            -> <Self as GraphSuccessors<'graph>>::Iter
    {
        self.successors[&node].iter().cloned()
    }
}

impl<'g> GraphPredecessors<'g> for MirCfg {
    type Item = NodeType;
    type Iter = iter::Cloned<slice::Iter<'g, NodeType>>;
}

impl<'g>  GraphSuccessors<'g> for MirCfg {
   type Item = NodeType;
    type Iter = iter::Cloned<slice::Iter<'g, NodeType>>;
}

//impl NodeIndex for BasicBlock {
//    fn as_usize(self) -> usize {
//        self.index()
//    }
//}
//impl From<usize> for BasicBlock {
//    fn from<usize>(n: usize) -> BasicBlock {
//        BasicBlock(n)
//    }
//}
