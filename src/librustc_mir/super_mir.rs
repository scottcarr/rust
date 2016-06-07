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

pub trait Graph
    where Self: for<'graph> GraphPredecessors<'graph, Item=<Self as Graph>::Node>,
          Self: for<'graph> GraphSuccessors<'graph, Item=<Self as Graph>::Node>
{
    type Node: NodeIndex;

    fn num_nodes(&self) -> usize;
    fn start_node(&self) -> Self::Node;
    fn predecessors<'graph>(&'graph self, node: Self::Node)
                            -> <Self as GraphPredecessors<'graph>>::Iter;
    fn successors<'graph>(&'graph self, node: Self::Node)
                            -> <Self as GraphSuccessors<'graph>>::Iter;
}

pub trait GraphPredecessors<'graph> {
    type Item;
    type Iter: Iterator<Item=Self::Item>;
}

pub trait GraphSuccessors<'graph> {
    type Item;
    type Iter: Iterator<Item=Self::Item>;
}

pub trait NodeIndex: Copy + Debug + Eq + Ord + Hash + Into<usize> + From<usize> {
    fn as_usize(self) -> usize {
        self.into()
    }
}

type NodeType = BasicBlock;
//type IndexType = BasicBlock;

fn compute_predecessors<'a, 'tcx>(mir: &'a Mir<'tcx>) -> HashMap<NodeType, Vec<NodeType>> {
    let mut predecessors = HashMap::new();
    for (from, data) in traversal::preorder(mir) {
        if let Some(ref term) = data.terminator {
            for &tgt in term.successors().iter() {
                predecessors.entry(tgt).or_insert(vec![]).push(from);
            }
        }
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
    successors
}

fn count_nodes<'a, 'tcx>(mir: &'a Mir<'tcx>) -> usize { mir.basic_blocks.len() }
    
impl SuperMir {
    fn new<'a, 'tcx>(mir: &'a Mir<'tcx>) -> Self {
        SuperMir { 
            predecessors: HashMap::new(), 
            successors: HashMap::new(), 
            n_nodes: count_nodes(mir), 
            start_node: BasicBlock::new(0), // Scott: is there some better way of setting this?
        }
    }
}

struct SuperMir {
    predecessors: HashMap<NodeType,Vec<NodeType>>,
    successors: HashMap<NodeType,Vec<NodeType>>,
    start_node: NodeType,
    n_nodes: usize,
}

impl Graph for SuperMir {

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

impl<'g> GraphPredecessors<'g> for SuperMir {
    type Item = NodeType;
    type Iter = iter::Cloned<slice::Iter<'g, NodeType>>;
}

impl<'g>  GraphSuccessors<'g> for SuperMir {
   type Item = NodeType;
    type Iter = iter::Cloned<slice::Iter<'g, NodeType>>;
}

impl NodeIndex for BasicBlock {
    fn as_usize(self) -> usize {
        self.index()
    }
}
//impl From<usize> for BasicBlock {
//    fn from<usize>(n: usize) -> BasicBlock {
//        BasicBlock(n)
//    }
//}
