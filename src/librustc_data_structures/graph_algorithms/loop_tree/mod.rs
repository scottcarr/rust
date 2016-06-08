// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use super::Graph;
use super::dominators::{Dominators, dominators};


#[cfg(test)]
mod test;
mod tree;
mod walk;

pub use self::tree::LoopTree;

pub fn loop_tree<G: Graph>(graph: &G) -> LoopTree<G> {
    let dominators = dominators(graph);
    loop_tree_given(graph, &dominators)
}

pub fn loop_tree_given<G: Graph>(graph: &G,
                                 dominators: &Dominators<G>)
                                 -> LoopTree<G>
{
    walk::LoopTreeWalk::new(graph, dominators).compute_loop_tree()
}


