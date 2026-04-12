#![doc = include_str!("../README.md")]

use std::{
	collections::{BTreeMap, HashMap},
	hash::Hash,
	ops::{Bound, Range},
};

use smallvec::SmallVec;

const DEFAULT_N: usize = 4;

/// A simple set supported by [`SmallVec`].
#[derive(Debug, Clone)]
struct Fencepost<K, const N: usize> {
	keys: SmallVec<[K; N]>,
}
impl<K, const N: usize> Fencepost<K, N> {
	fn as_slice(&self) -> &[K] {
		&self.keys
	}
}
impl<K: Eq, const N: usize> Fencepost<K, N> {
	fn insert(&mut self, key: K) {
		if !self.keys.contains(&key) {
			self.keys.push(key);
		}
	}
	fn remove(&mut self, key: &K) {
		if let Some(del) = self.keys.iter().position(|k| k == key) {
			self.keys.swap_remove(del);
		}
	}
}
impl<K, const N: usize> Default for Fencepost<K, N> {
	fn default() -> Self {
		Self {
			keys: Default::default(),
		}
	}
}

/// A range map for overlapping ranges of index `I` mapping to `V`.
#[derive(Debug, Clone)]
pub struct MultiRangeMap<I, V, const N: usize = DEFAULT_N> {
	posts: BTreeMap<I, Fencepost<V, N>>,
}
impl<I: Ord, V, const N: usize> MultiRangeMap<I, V, N> {
	/// Returns the first key values.
	pub fn first(&self) -> Option<&I> {
		let (start, _) = self.posts.first_key_value()?;
		Some(start)
	}
	/// Returns the last key values.
	pub fn last(&self) -> Option<&I> {
		let (end, _) = self.posts.last_key_value()?;
		Some(end)
	}
	/// Returns the first and last key values.
	pub fn bounds(&self) -> Option<(&I, &I)> {
		let start = self.first()?;
		let end = self.last()?;
		Some((start, end))
	}
	/// Returns `true` if `i` is contained between [`bounds`](Self::bounds).
	pub fn contains(&self, i: &I) -> bool {
		self.bounds()
			.is_some_and(|(start, end)| start <= i && i < end)
	}

	/// Get values at key `i`.
	///
	/// These are the values of the previous range bound.
	pub fn get(&self, i: &I) -> Option<&[V]> {
		self.get_prev_post(i).map(Fencepost::as_slice)
	}
	/// Get values of the range bound before 'i'.
	pub fn get_prev(&self, i: &I) -> Option<(&I, &[V])> {
		self.get_prev_key_post(i).map(|(i, v)| (i, v.as_slice()))
	}
	/// Get values of the range bound after 'i'.
	pub fn get_next(&self, i: &I) -> Option<(&I, &[V])> {
		self.get_next_key_post(i).map(|(i, v)| (i, v.as_slice()))
	}

	/// Get previous post.
	fn get_prev_post(&self, i: &I) -> Option<&Fencepost<V, N>> {
		let (_, post) = self.get_prev_key_post(i)?;
		Some(post)
	}

	/// Get previous post and key.
	fn get_prev_key_post(&self, i: &I) -> Option<(&I, &Fencepost<V, N>)> {
		let mut range = self.posts.range(..=i);
		range.next_back()
	}
	/// Get next post and key.
	fn get_next_key_post(&self, i: &I) -> Option<(&I, &Fencepost<V, N>)> {
		let mut range = self.posts.range(i..);
		range.next_back()
	}

	/// Get previous post mutably, if exactly at `i`.
	fn get_prev_post_exactly_mut(&mut self, i: &I) -> Option<&mut Fencepost<V, N>> {
		self.posts.get_mut(i)
	}

	/// Returns true if `i` is the key of a post.
	fn is_post(&self, i: &I) -> bool {
		self.posts.contains_key(i)
	}
}
impl<I: Ord + Clone, V: Clone, const N: usize> MultiRangeMap<I, V, N> {
	fn insert_post(&mut self, i: I) -> &mut Fencepost<V, N> {
		let prev = self.get_prev_post(&i).map(Clone::clone).unwrap_or_default();
		self.posts.entry(i).insert_entry(prev).into_mut()
	}
	fn get_or_insert_post(&mut self, i: I) -> &mut Fencepost<V, N> {
		if self.is_post(&i) {
			return self.get_prev_post_exactly_mut(&i).unwrap();
		}
		self.insert_post(i)
	}
}
impl<I: Ord + Clone, V: Eq + Clone, const N: usize> MultiRangeMap<I, V, N> {
	/// Ranges can overlap and a given index `I` may map onto multiple values.
	pub fn insert(&mut self, range: Range<I>, v: V) {
		let (start, end) = (range.start, range.end);
		for (_, post) in self
			.posts
			.range_mut((Bound::Excluded(start.clone()), Bound::Excluded(end.clone())))
		{
			post.insert(v.clone());
		}
		let start = self.get_or_insert_post(start);
		start.insert(v);
	}
	/// - Removing exactly the ranges which `v` was inserted with will
	/// remove the value completely.
	/// - Removing a subrange will create new posts on either side of
	/// the range which exclude the value. This separates the ranges, but
	/// still maps both of the new ranges to the value.
	/// - Removing a partially overlapping range will trim the range.
	pub fn remove(&mut self, range: Range<I>, v: &V) {
		let (start, end) = (range.start, range.end);
		for (_, post) in self
			.posts
			.range_mut((Bound::Excluded(start.clone()), Bound::Excluded(end.clone())))
		{
			post.remove(v);
		}
		let start = self.get_or_insert_post(end);
		start.remove(v);
	}
}
impl<I, V, const N: usize> Default for MultiRangeMap<I, V, N> {
	fn default() -> Self {
		Self {
			posts: Default::default(),
		}
	}
}

/// A range map for overlapping ranges of index `I` mapping to `K` and vice versa using [`MultiRangeMap`] and [`HashMap`].
#[derive(Debug, Clone)]
pub struct MultiRangeHashMap<I, K, const N: usize = DEFAULT_N> {
	fence: MultiRangeMap<I, K, N>,
	bounds: HashMap<K, Range<I>>,
}
impl<I: Ord, K, const N: usize> MultiRangeHashMap<I, K, N> {
	/// Returns the first index.
	pub fn first_index(&mut self) -> Option<&I> {
		self.fence.first()
	}
	/// Returns the last index.
	pub fn last_index(&mut self) -> Option<&I> {
		self.fence.last()
	}
	/// Returns the first and last index.
	pub fn bounds(&mut self) -> Option<(&I, &I)> {
		self.fence.bounds()
	}
}
impl<I: Ord + Clone, K: Hash + Eq + Clone, const N: usize> MultiRangeHashMap<I, K, N> {
	pub fn insert(&mut self, k: K, range: Range<I>) {
		self.bounds.insert(k.clone(), range.clone());
		self.fence.insert(range, k);
	}
	pub fn remove(&mut self, k: &K) {
		if let Some(range) = self.bounds.remove(&k) {
			self.fence.remove(range, &k);
		}
	}
}
impl<I, K: Hash + Eq, const N: usize> MultiRangeHashMap<I, K, N> {
	pub fn get(&self, k: &K) -> Option<&Range<I>> {
		self.bounds.get(k)
	}
}
impl<I: Ord, K, const N: usize> MultiRangeHashMap<I, K, N> {
	/// Returns the keys of `i`.
	pub fn index(&self, i: &I) -> Option<&[K]> {
		self.fence.get(i)
	}
	/// Returns the keys of the range bound before `i`.
	pub fn index_prev(&self, i: &I) -> Option<(&I, &[K])> {
		self.fence.get_prev(i)
	}
	/// Returns the keys of the range bound after `i`.
	pub fn index_next(&self, i: &I) -> Option<(&I, &[K])> {
		self.fence.get_next(i)
	}
}
impl<I, V, const N: usize> Default for MultiRangeHashMap<I, V, N> {
	fn default() -> Self {
		Self {
			fence: Default::default(),
			bounds: Default::default(),
		}
	}
}

/// A range map for overlapping ranges of index `I` mapping to `K` and vice versa using [`MultiRangeMap`] and [`BTreeMap`].
#[derive(Debug, Clone)]
pub struct MultiRangeBTreeMap<I, K, const N: usize = DEFAULT_N> {
	fence: MultiRangeMap<I, K, N>,
	bounds: BTreeMap<K, Range<I>>,
}
impl<I: Ord, K, const N: usize> MultiRangeBTreeMap<I, K, N> {
	/// Returns the first index.
	pub fn first_index(&mut self) -> Option<&I> {
		self.fence.first()
	}
	/// Returns the last index.
	pub fn last_index(&mut self) -> Option<&I> {
		self.fence.last()
	}
	/// Returns the first and last index.
	pub fn bounds(&mut self) -> Option<(&I, &I)> {
		self.fence.bounds()
	}
}
impl<I: Ord + Clone, K: Ord + Clone, const N: usize> MultiRangeBTreeMap<I, K, N> {
	pub fn insert(&mut self, key: K, range: Range<I>) {
		self.bounds.insert(key.clone(), range.clone());
		self.fence.insert(range, key);
	}
	pub fn remove(&mut self, key: &K) {
		if let Some(range) = self.bounds.remove(&key) {
			self.fence.remove(range, &key);
		}
	}
}
impl<I, K: Ord, const N: usize> MultiRangeBTreeMap<I, K, N> {
	pub fn get(&self, k: &K) -> Option<&Range<I>> {
		self.bounds.get(k)
	}
}
impl<I: Ord, K, const N: usize> MultiRangeBTreeMap<I, K, N> {
	/// Returns the keys of `i`.
	pub fn index(&self, i: &I) -> Option<&[K]> {
		self.fence.get(i)
	}
	/// Returns the keys of the range bound before `i`.
	pub fn index_prev(&self, i: &I) -> Option<(&I, &[K])> {
		self.fence.get_prev(i)
	}
	/// Returns the keys of the range bound after `i`.
	pub fn index_next(&self, i: &I) -> Option<(&I, &[K])> {
		self.fence.get_next(i)
	}
}
impl<I, V, const N: usize> Default for MultiRangeBTreeMap<I, V, N> {
	fn default() -> Self {
		Self {
			fence: Default::default(),
			bounds: Default::default(),
		}
	}
}

#[cfg(any(feature = "slotmap"))]
pub use multirangeslotmap::*;
#[cfg(any(feature = "slotmap"))]
mod multirangeslotmap {
	use std::ops::Range;

	pub use slotmap;
	use slotmap::{Key, SlotMap};

	use crate::{DEFAULT_N, MultiRangeMap};

	/// A range map for overlapping ranges of index `I` mapping to `K` and vice versa using [`MultiRangeMap`] and [`SlotMap`].
	pub struct MultiRangeSlotMap<I, K: Key, const N: usize = DEFAULT_N> {
		fence: MultiRangeMap<I, K, N>,
		bounds: SlotMap<K, Range<I>>,
	}
	impl<I: Ord, K: Key, const N: usize> MultiRangeSlotMap<I, K, N> {
		/// Returns the first index.
		pub fn first_index(&mut self) -> Option<&I> {
			self.fence.first()
		}
		/// Returns the last index.
		pub fn last_index(&mut self) -> Option<&I> {
			self.fence.last()
		}
		/// Returns the first and last index.
		pub fn bounds(&mut self) -> Option<(&I, &I)> {
			self.fence.bounds()
		}
	}
	impl<I: Ord + Clone, K: Key, const N: usize> MultiRangeSlotMap<I, K, N> {
		pub fn insert(&mut self, range: Range<I>) -> K {
			let key = self.bounds.insert(range.clone());
			self.fence.insert(range, key);
			key
		}
		pub fn remove(&mut self, key: K) {
			if let Some(range) = self.bounds.remove(key) {
				self.fence.remove(range, &key);
			}
		}
	}
	impl<I, K: Key, const N: usize> MultiRangeSlotMap<I, K, N> {
		pub fn get(&self, k: K) -> Option<&Range<I>> {
			self.bounds.get(k)
		}
	}
	impl<I: Ord, K: Key, const N: usize> MultiRangeSlotMap<I, K, N> {
		/// Returns the keys of `i`.
		pub fn index(&self, i: &I) -> Option<&[K]> {
			self.fence.get(i)
		}
		/// Returns the keys of the range bound before `i`.
		pub fn index_prev(&self, i: &I) -> Option<(&I, &[K])> {
			self.fence.get_prev(i)
		}
		/// Returns the keys of the range bound after `i`.
		pub fn index_next(&self, i: &I) -> Option<(&I, &[K])> {
			self.fence.get_next(i)
		}
	}
	impl<I, V: Key, const N: usize> Default for MultiRangeSlotMap<I, V, N> {
		fn default() -> Self {
			Self {
				fence: Default::default(),
				bounds: Default::default(),
			}
		}
	}
}
