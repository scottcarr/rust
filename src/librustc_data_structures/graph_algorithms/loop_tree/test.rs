// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use test::TestGraph;
use super::loop_tree;

#[test]
fn test1() {
    // 0 -> 1 -> 2 -> 3
    //      ^    v
    //      6 <- 4 -> 5
    let graph = TestGraph::new(0, &[
        (0, 1),
        (1, 2),
        (2, 3),
        (2, 4),
        (4, 5),
        (4, 6),
        (6, 1),
    ]);
    let loop_tree = loop_tree(&graph);
    assert_eq!(loop_tree.loop_head_of_node(0), None);
    assert_eq!(loop_tree.loop_head_of_node(1), Some(1));
    assert_eq!(loop_tree.loop_head_of_node(2), Some(1));
    assert_eq!(loop_tree.loop_head_of_node(3), None);
    assert_eq!(loop_tree.loop_head_of_node(4), Some(1));
    assert_eq!(loop_tree.loop_head_of_node(5), None);
    assert_eq!(loop_tree.loop_head_of_node(6), Some(1));

    let loop_id = loop_tree.loop_id(1).unwrap();
    assert_eq!(loop_tree.loop_id(2), Some(loop_id));
    assert_eq!(loop_tree.parent(loop_id), None);
    assert_eq!(loop_tree.loop_exits(loop_id), &[3, 5]);
}

#[test]
fn nested_loop() {
    // 0 -> 1 ->     2     -> 3 -> 5
    //      ^     ^    v      |
    //      |     6 <- 4      |
    //      +-----------------+
    let graph = TestGraph::new(0, &[
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 5),
        (3, 1),
        (2, 4),
        (4, 6),
        (6, 2),
    ]);
    let loop_tree = loop_tree(&graph);
    assert_eq!(loop_tree.loop_head_of_node(0), None);
    assert_eq!(loop_tree.loop_head_of_node(1), Some(1));
    assert_eq!(loop_tree.loop_head_of_node(2), Some(2));
    assert_eq!(loop_tree.loop_head_of_node(3), Some(1));
    assert_eq!(loop_tree.loop_head_of_node(4), Some(2));
    assert_eq!(loop_tree.loop_head_of_node(5), None);
    assert_eq!(loop_tree.loop_head_of_node(6), Some(2));

    let outer_loop_id = loop_tree.loop_id(1).unwrap();
    let inner_loop_id = loop_tree.loop_id(2).unwrap();
    assert_eq!(loop_tree.parent(outer_loop_id), None);
    assert_eq!(loop_tree.parent(inner_loop_id), Some(outer_loop_id));

    assert_eq!(loop_tree.loop_exits(outer_loop_id), &[5]);
    assert_eq!(loop_tree.loop_exits(inner_loop_id), &[3]);
}


#[test]
fn if_else_break_nested_loop() {
    // 0 -> 1 ->     2     -> 3 -> 5
    //      ^     ^    v      |    ^
    //      |     6 <- 4      |    |
    //      |          |      |    |
    //      |     7 <--+      |    |
    //      +-----|-----------+    |
    //            +----------------+
    let graph = TestGraph::new(0, &[
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 5),
        (3, 1),
        (2, 4),
        (4, 6),
        (4, 7),
        (6, 2),
        (7, 5),
    ]);
    let loop_tree = loop_tree(&graph);
    assert_eq!(loop_tree.loop_head_of_node(0), None);
    assert_eq!(loop_tree.loop_head_of_node(1), Some(1));
    assert_eq!(loop_tree.loop_head_of_node(2), Some(2));
    assert_eq!(loop_tree.loop_head_of_node(3), Some(1));
    assert_eq!(loop_tree.loop_head_of_node(4), Some(2));
    assert_eq!(loop_tree.loop_head_of_node(5), None);
    assert_eq!(loop_tree.loop_head_of_node(6), Some(2));
    assert_eq!(loop_tree.loop_head_of_node(7), None);

    let outer_loop_id = loop_tree.loop_id(1).unwrap();
    let inner_loop_id = loop_tree.loop_id(2).unwrap();
    assert_eq!(loop_tree.parent(outer_loop_id), None);
    assert_eq!(loop_tree.parent(inner_loop_id), Some(outer_loop_id));

    assert_eq!(loop_tree.loop_exits(outer_loop_id), &[7, 5]);
    assert_eq!(loop_tree.loop_exits(inner_loop_id), &[3, 7]);
}

#[test]
fn wacked() {
    // This example looks kind of mind-bending,
    // but really isn't. It could result from some code like:
    //
    //     loop {
    //         if ... {
    //             continue
    //         } else {
    //             continue
    //         }
    //     }
    //
    // It came from Munchnick's book.
    //
    // +-1
    // v/
    // 0--->3
    // ^\
    // +-2
    let graph = TestGraph::new(0, &[
        (0, 1),
        (1, 0),
        (0, 2),
        (2, 0),
        (0, 3),
    ]);
    let loop_tree = loop_tree(&graph);
    assert_eq!(loop_tree.loop_head_of_node(0), Some(0));
    assert_eq!(loop_tree.loop_head_of_node(1), Some(0));
    assert_eq!(loop_tree.loop_head_of_node(2), Some(0));
    assert_eq!(loop_tree.loop_head_of_node(3), None);

    let outer_loop_id = loop_tree.loop_id(0).unwrap();
    assert_eq!(loop_tree.loop_exits(outer_loop_id), &[3]);
}
