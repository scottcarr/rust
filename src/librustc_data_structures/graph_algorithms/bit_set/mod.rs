// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::marker::PhantomData;
use std::mem;

use super::{Graph, NodeIndex};

type Word = u32;

pub struct BitSet<G: Graph> {
    bits_per_node: usize,
    words: Vec<Word>,
    graph: PhantomData<G>,
}

impl<G: Graph> BitSet<G> {
    pub fn new(graph: &G, bits_per_node: usize) -> Self {
        let num_nodes = graph.num_nodes();
        let words_per_node = words(bits_per_node);
        let words = vec![0; words_per_node * num_nodes];
        BitSet {
            bits_per_node: bits_per_node,
            words: words,
            graph: PhantomData,
        }
    }

    fn index(&self, node: G::Node) -> usize {
        node.as_usize() * words(self.bits_per_node)
    }

    pub fn is_set(&self, node: G::Node, bit: usize) -> bool {
        let start = self.index(node);
        let (word, bit) = words_bits(bit);
        let value = self.words[start + word];
        (value & (1 << bit)) != 0
    }

    pub fn insert(&mut self, node: G::Node, bit: usize) -> bool {
        let start = self.index(node);
        let (word, bit) = words_bits(bit);
        let old_value = self.words[start + word];
        let new_value = old_value | (1 << bit);
        self.words[start + word] = new_value;
        old_value != new_value
    }

    pub fn insert_bits_from(&mut self,
                            source_node: G::Node,
                            target_node: G::Node)
                            -> bool {
        if source_node == target_node {
            return false;
        }
        let words_per_node = words(self.bits_per_node);
        let source_start = source_node.as_usize() * words_per_node;
        let target_start = target_node.as_usize() * words_per_node;
        let mut changed = false;
        for offset in 0..words_per_node {
            let source_word = self.words[source_start + offset];
            let target_word = self.words[target_start + offset];
            let new_word = source_word | target_word;
            self.words[target_start + offset] = new_word;
            changed |= new_word != target_word;
        }
        changed
    }
}

#[inline]
fn words_bits(x: usize) -> (usize, usize) {
    let d = mem::size_of::<Word>() * 8;
    (x / d, x % d)
}

#[inline]
fn words(x: usize) -> usize {
    let (w, b) = words_bits(x);
    if b != 0 {w + 1} else {w}
}

