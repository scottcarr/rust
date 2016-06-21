// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// error-pattern: overflow representing the type `S`

#![feature(rustc_attrs)]

trait Mirror { type It: ?Sized; }
impl<T: ?Sized> Mirror for T { type It = Self; }
struct S(Option<<S as Mirror>::It>);

#[rustc_no_mir] // FIXME #27840 MIR tries to represent `std::option::Option<S>` first.
fn main() {
    let _s = S(None);
}
