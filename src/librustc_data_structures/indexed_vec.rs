// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::iter::{self, FromIterator};
use std::slice;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut, Range};
use std::fmt;
use std::vec;
use std::hash::Hash;
use std::fmt::Debug;

use rustc_serialize as serialize;

/// Represents some newtyped `usize` wrapper.
///
/// (purpose: avoid mixing indexes for different bitvector domains.)
pub trait NodeIndex: Copy + Debug + Eq + Ord + Hash + Into<usize> + From<usize> + 'static {
}

#[derive(Clone)]
pub struct IndexVec<I: NodeIndex, T> {
    pub raw: Vec<T>,
    _marker: PhantomData<Fn(&I)>
}

impl<I: NodeIndex, T: serialize::Encodable> serialize::Encodable for IndexVec<I, T> {
    fn encode<S: serialize::Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        serialize::Encodable::encode(&self.raw, s)
    }
}

impl<I: NodeIndex, T: serialize::Decodable> serialize::Decodable for IndexVec<I, T> {
    fn decode<D: serialize::Decoder>(d: &mut D) -> Result<Self, D::Error> {
        serialize::Decodable::decode(d).map(|v| {
            IndexVec { raw: v, _marker: PhantomData }
        })
    }
}

impl<I: NodeIndex, T: fmt::Debug> fmt::Debug for IndexVec<I, T> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.raw, fmt)
    }
}

pub type Enumerated<I, J> = iter::Map<iter::Enumerate<J>, IntoNodeIndex<I>>;

impl<I: NodeIndex, T> IndexVec<I, T> {
    #[inline]
    pub fn new() -> Self {
        IndexVec { raw: Vec::new(), _marker: PhantomData }
    }

    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        IndexVec { raw: Vec::with_capacity(capacity), _marker: PhantomData }
    }

    #[inline]
    pub fn from_elem<S>(elem: T, universe: &IndexVec<I, S>) -> Self
        where T: Clone
    {
        IndexVec { raw: vec![elem; universe.len()], _marker: PhantomData }
    }

    #[inline]
    pub fn push(&mut self, d: T) -> I {
        let idx = self.len();
        self.raw.push(d);
        I::from(idx)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.raw.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.raw.is_empty()
    }

    #[inline]
    pub fn into_iter(self) -> vec::IntoIter<T> {
        self.raw.into_iter()
    }

    #[inline]
    pub fn into_iter_enumerated(self) -> Enumerated<I, vec::IntoIter<T>>
    {
        self.raw.into_iter().enumerate().map(IntoNodeIndex { _marker: PhantomData })
    }

    #[inline]
    pub fn iter(&self) -> slice::Iter<T> {
        self.raw.iter()
    }

    #[inline]
    pub fn iter_enumerated(&self) -> Enumerated<I, slice::Iter<T>>
    {
        self.raw.iter().enumerate().map(IntoNodeIndex { _marker: PhantomData })
    }

    #[inline]
    pub fn indices(&self) -> iter::Map<Range<usize>, IntoNodeIndex<I>> {
        (0..self.len()).map(IntoNodeIndex { _marker: PhantomData })
    }

    #[inline]
    pub fn iter_mut(&mut self) -> slice::IterMut<T> {
        self.raw.iter_mut()
    }

    #[inline]
    pub fn iter_enumerated_mut(&mut self) -> Enumerated<I, slice::IterMut<T>>
    {
        self.raw.iter_mut().enumerate().map(IntoNodeIndex { _marker: PhantomData })
    }

    #[inline]
    pub fn last(&self) -> Option<I> {
        match self.len().checked_sub(1) {
            Some (x) => Some(I::from(x)),
            None => None
        }
    }
}

impl<I: NodeIndex, T> Index<I> for IndexVec<I, T> {
    type Output = T;

    #[inline]
    fn index(&self, index: I) -> &T {
        &self.raw[index.into()]
    }
}

impl<I: NodeIndex, T> IndexMut<I> for IndexVec<I, T> {
    #[inline]
    fn index_mut(&mut self, index: I) -> &mut T {
        &mut self.raw[index.into()]
    }
}

impl<I: NodeIndex, T> Extend<T> for IndexVec<I, T> {
    #[inline]
    fn extend<J: IntoIterator<Item = T>>(&mut self, iter: J) {
        self.raw.extend(iter);
    }
}

impl<I: NodeIndex, T> FromIterator<T> for IndexVec<I, T> {
    #[inline]
    fn from_iter<J>(iter: J) -> Self where J: IntoIterator<Item=T> {
        IndexVec { raw: FromIterator::from_iter(iter), _marker: PhantomData }
    }
}

impl<I: NodeIndex, T> IntoIterator for IndexVec<I, T> {
    type Item = T;
    type IntoIter = vec::IntoIter<T>;

    #[inline]
    fn into_iter(self) -> vec::IntoIter<T> {
        self.raw.into_iter()
    }

}

impl<'a, I: NodeIndex, T> IntoIterator for &'a IndexVec<I, T> {
    type Item = &'a T;
    type IntoIter = slice::Iter<'a, T>;

    #[inline]
    fn into_iter(self) -> slice::Iter<'a, T> {
        self.raw.iter()
    }
}

impl<'a, I: NodeIndex, T> IntoIterator for &'a mut IndexVec<I, T> {
    type Item = &'a mut T;
    type IntoIter = slice::IterMut<'a, T>;

    #[inline]
    fn into_iter(mut self) -> slice::IterMut<'a, T> {
        self.raw.iter_mut()
    }
}

pub struct IntoNodeIndex<I: NodeIndex> { _marker: PhantomData<fn(&I)> }
impl<I: NodeIndex, T> FnOnce<((usize, T),)> for IntoNodeIndex<I> {
    type Output = (I, T);

    extern "rust-call" fn call_once(self, ((n, t),): ((usize, T),)) -> Self::Output {
        (I::from(n), t)
    }
}

impl<I: NodeIndex, T> FnMut<((usize, T),)> for IntoNodeIndex<I> {
    extern "rust-call" fn call_mut(&mut self, ((n, t),): ((usize, T),)) -> Self::Output {
        (I::from(n), t)
    }
}

impl<I: NodeIndex> FnOnce<(usize,)> for IntoNodeIndex<I> {
    type Output = I;

    extern "rust-call" fn call_once(self, (n,): (usize,)) -> Self::Output {
        I::from(n)
    }
}

impl<I: NodeIndex> FnMut<(usize,)> for IntoNodeIndex<I> {
    extern "rust-call" fn call_mut(&mut self, (n,): (usize,)) -> Self::Output {
        I::from(n)
    }
}

