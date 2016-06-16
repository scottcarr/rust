// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use super::indexed_vec::{Idx, IndexVec};
use core::marker::{PhantomData};
pub use std::slice::Iter;
use std::ops::{Index, IndexMut};
use std::clone::Clone;
use std;

//pub mod bit_set;
pub mod dominators;
pub mod iterate;
pub mod reachable;
mod reference;
//pub mod node_vec;
pub mod transpose;

#[cfg(test)]
mod test;

pub trait Graph
    where Self: for<'graph> GraphPredecessors<'graph, Item=<Self as Graph>::Node>,
          Self: for<'graph> GraphSuccessors<'graph, Item=<Self as Graph>::Node>
    //where Self: Sized
{
    type Node: Idx;

    fn num_nodes(&self) -> usize;
    fn start_node(&self) -> Self::Node;
    fn predecessors<'graph>(&'graph self, node: Self::Node)
                            -> <Self as GraphPredecessors<'graph>>::Iter;
                            // why is returning an iterator so complicated?
                            //-> NodeVec<Self, Self::Node>;
    fn successors<'graph>(&'graph self, node: Self::Node)
                            -> <Self as GraphSuccessors<'graph>>::Iter;
                            //-> NodeVec<Self, Self::Node>;
                            //-> std::slice::Iter<'graph, Self::Node>;
    //fn from_default(&self) -> IndexVec<Self::Node, Self::Node> {
    //    (0..self.num_nodes()).map(|| Self::Node::default()).collect()
    //}
}

pub trait GraphPredecessors<'graph> {
    type Item;
    type Iter: Iterator<Item=Self::Item>;
}

pub trait GraphSuccessors<'graph> {
    type Item;
    type Iter: Iterator<Item=Self::Item>;
}

//pub trait NodeIndex: Copy + Debug + Eq + Ord + Hash + Into<usize> + From<usize> {
//    fn as_usize(self) -> usize {
//        self.into()
//    }
//}

//#[derive(Clone, Debug)]
//pub struct NodeVec<G: Graph, T> {
//    pub vec: Vec<T>,
//    graph: PhantomData<G>,
//}
//
//impl<G: Graph, T: Clone> NodeVec<G, T> {
//    pub fn from_elem(graph: &G, default: &T) -> Self {
//        NodeVec::from_fn(graph, |_| default.clone())
//    }
//
//    pub fn from_elem_with_len(num_nodes: usize, default: &T) -> Self {
//        NodeVec::from_fn_with_len(num_nodes, |_| default.clone())
//    }
//}
//
//impl<G: Graph, T: Default> NodeVec<G, T> {
//    pub fn from_default(graph: &G) -> Self {
//        NodeVec::from_fn(graph, |_| T::default())
//    }
//
//    pub fn from_default_with_len(num_nodes: usize) -> Self {
//        NodeVec::from_fn_with_len(num_nodes, |_| T::default())
//    }
//}
//
//impl<G: Graph, T> NodeVec<G, T> {
//    pub fn from_vec(v: Vec<T>) -> Self 
//        where T: Clone
//    {
//
//        NodeVec {
//            vec: v.clone(),
//            graph: PhantomData,
//        }
//    }
//
//    pub fn from_fn<F>(graph: &G, f: F) -> Self
//        where F: FnMut(G::Node) -> T
//    {
//        Self::from_fn_with_len(graph.num_nodes(), f)
//    }
//
//    pub fn from_fn_with_len<F>(num_nodes: usize, f: F) -> Self
//        where F: FnMut(G::Node) -> T
//    {
//        NodeVec {
//            vec: (0..num_nodes).map(G::Node::new).map(f).collect(),
//            graph: PhantomData,
//        }
//    }
//
//    pub fn iter(&self) -> Iter<T> {
//        self.vec.iter()
//    }
//
//    pub fn len(&self) -> usize {
//        self.vec.len()
//    }
//}
//
//impl<G: Graph, T> Index<G::Node> for NodeVec<G, T> {
//    type Output = T;
//
//    fn index(&self, index: G::Node) -> &T {
//        let index: usize = index.index();
//        &self.vec[index]
//    }
//}
//
//impl<G: Graph, T> IndexMut<G::Node> for NodeVec<G, T> {
//    fn index_mut(&mut self, index: G::Node) -> &mut T {
//        let index: usize = index.index();
//        &mut self.vec[index]
//    }
//}

