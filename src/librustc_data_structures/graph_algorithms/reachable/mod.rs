// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Compute reachability using a simple dataflow propagation.
//! Store end-result in a big NxN bit matrix.

use super::{Graph, NodeIndex};
//use super::bit_set::BitSet;
use super::super::bitvec::BitVector;
use super::iterate::reverse_post_order;

#[cfg(test)]
mod test;

pub fn reachable<G: Graph>(graph: &G)
                           -> Reachability {
    let reverse_post_order = reverse_post_order(graph, graph.start_node());
    reachable_given_rpo(graph, &reverse_post_order)
}

pub fn reachable_given_rpo<G: Graph>(graph: &G,
                                     reverse_post_order: &[G::Node])
                                     -> Reachability {
    let mut reachability = Reachability::new(graph);
    let mut changed = true;
    while changed {
        changed = false;
        for &node in reverse_post_order.iter().rev() {
            // every node can reach itself
            //changed |= reachability.bits.insert(node, node.as_usize());
            changed |= reachability.bits[node.as_usize()].insert(node.as_usize());

            // and every pred can reach everything node can reach
            for pred in graph.predecessors(node) {
                //changed |= reachability.bits.insert_bits_from(node, pred);
                changed |= reachability.bits[node.as_usize()].insert(pred.as_usize());
            }
        }
    }
    reachability
}

//pub struct Reachability<G: Graph> {
pub struct Reachability {
    //bits: BitSet<G>,
    bits: Vec<BitVector>,
}

//impl<G: Graph> Reachability {
impl Reachability {
    fn new<G: Graph>(graph: &G) -> Self {
        let num_nodes = graph.num_nodes();
        Reachability {
            //bits: BitSet::new(graph, num_nodes),
            bits: vec![BitVector::new(num_nodes)],
        }
    }

    pub fn can_reach<G: Graph>(&self, source: G::Node, target: G::Node) -> bool {
        let bit: usize = target.into();
        //self.bits.is_set(source, bit)
        self.bits[source.as_usize()].contains(bit)
    }
}
