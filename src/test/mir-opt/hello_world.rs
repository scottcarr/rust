// Copyright 2012-2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.



fn main() {
    println!("hello_world");
}

// END RUST SOURCE
// START rustc.node4.ElaborateDrops.after.mir
// // MIR for `main`
// // node_id = 4
// // pass_name = ElaborateDrops
// // disambiguator = after
// fn main() -> () {
//     let mut tmp0: ();
//     let mut tmp1: std::fmt::Arguments;
//     let mut tmp2: &[&str];
//     let mut tmp3: &[std::fmt::ArgumentV1];
//     let mut tmp4: &[std::fmt::ArgumentV1; 0];
//     let mut tmp5: &[std::fmt::ArgumentV1; 0];
//     let mut tmp6: [std::fmt::ArgumentV1; 0];
//     let mut tmp7: ();
//     bb0: {
//         tmp2 = &(*main::__STATIC_FMTSTR); // scope 0 at <std macros>:1:33: 1:58
//         tmp7 = ();                       // scope 0 at <std macros>:1:33: 1:58
//         goto -> bb1;                     // scope 0 at <std macros>:1:33: 1:58
//     }
//     bb1: {
//         tmp5 = promoted0;                // scope 0 at <std macros>:1:33: 1:58
//         tmp4 = &(*tmp5);                 // scope 0 at <std macros>:1:33: 1:58
//         tmp3 = tmp4 as &[std::fmt::ArgumentV1] (Unsize); // scope 0 at <std macros>:1:33: 1:58
//         tmp1 = std::fmt::Arguments::new_v1(tmp2, tmp3) -> bb2; // scope 0 at <std macros>:2:27: 2:58
//     }
//     bb2: {
//         tmp0 = std::io::_print(tmp1) -> bb3; // scope 0 at <std macros>:2:1: 2:60
//     }
//     bb3: {
//         return = ();                     // scope 0 at /Users/scarr/mozilla/rust-workspace/mir-test/rust/src/test/mir-opt/hello_world.rs:13:11: 15:2
//         return;                          // scope 0 at /Users/scarr/mozilla/rust-workspace/mir-test/rust/src/test/mir-opt/hello_world.rs:13:1: 15:2
//     }
//     bb4: {
//         resume;                          // scope 0 at /Users/scarr/mozilla/rust-workspace/mir-test/rust/src/test/mir-opt/hello_world.rs:13:1: 15:2
//     }
// }
// END rustc.node4.ElaborateDrops.after.mir