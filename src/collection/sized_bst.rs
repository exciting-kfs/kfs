use super::sized_pool::SizedPool;

struct Collision;

struct Node {
	idx: usize,
	parent: Option<usize>,
	left: Option<usize>,
	right: Option<usize>,
}

impl Node {
	fn with_index(idx: usize) -> Self {
		Node { idx, parent: None, left: None, right: None }
	}

	pub fn direct_mut<K>(&mut self, base: &K, target: &K) -> Result<&mut Option<usize>, Collision>
	where K: PartialOrd
	{
		if *target < *base {
			Ok(&mut self.left)
		} else if *target > *base {
			Ok(&mut self.right)
		} else {
			Err(Collision)
		}
	}

	pub fn direct<K>(&self, base: &K, target: &K) -> Result<&Option<usize>, Collision>
	where K: PartialOrd
	{
		if *target < *base {
			Ok(&self.left)
		} else if *target > *base {
			Ok(&self.right)
		} else {
			Err(Collision)
		}
	}
}

struct Pair<K, V> {
	key: K,
	value: V
}

impl<K, V> Pair<K, V>
where K: PartialEq
{
	fn with_index(_: usize) -> Self
	where K: Default, V: Default
	{
		Pair { key: K::default(), value: V::default() }
	}

	pub fn new(key: K, value: V) -> Self {
		Pair { key, value }
	}
}

enum Error {
	Duplicated,
	NotFound,
	Full,
}

struct SizedBST<K, V, const CAP: usize> {
	root: Option<usize>,
	relation: [Node; CAP],
	pool: SizedPool<Pair<K, V>, CAP>
}

impl<K, V, const CAP:usize> SizedBST<K, V, CAP>
where K: Clone + PartialOrd
{
	pub fn from_fn<F>(cb: F) -> Self
	where F: FnMut(usize) -> Pair<K, V>
	{
		SizedBST {
			root: None,
			relation: core::array::from_fn(Node::with_index),
			pool: SizedPool::<Pair<K, V>, CAP>::from_fn(cb)
		}
	}

	pub fn insert(&mut self, pair: Pair<K, V>) -> Result<(), Error> {
		if let None = self.root {
			self.pool.insert(pair);
			self.root = Some(0);
			return Ok(())
		}

		let key = pair.key.clone();
		let new_idx = self.pool.insert(pair).ok_or(Error::Full)?;
		let mut i = self.root.unwrap();
		
		loop {
			let base = self.get_key(i).unwrap();
			let curr = &mut self.relation[i];
			let arm = curr
				.direct_mut(&base, &key)
				.map_err(|_| Error::Duplicated)?;
			match arm {
				Some(child_idx) => i = *child_idx,
				None => {
					*arm = Some(new_idx);
					let curr_idx = curr.idx;
					let new_node = &mut self.relation[new_idx];
					new_node.parent = Some(curr_idx);
					break;
				}
			}
		}
		Ok(())
	}

	fn get_key(&self, idx: usize) -> Option<K> {
		self.pool.at(idx).map(|pair| pair.key.clone())
	}


	fn get_value(&self, idx: usize) -> Option<&V> {
		self.pool.at(idx).map(|pair| &pair.value)
	}

	pub fn remove(&mut self, key: &K) -> Result<V, Error> { // 여기부터..
		self.find(key).map(|idx| {
			let curr = &mut self.relation[idx];
			let sucessor = curr.right.map(|r| self.leftmost(r));
			let sucessor = sucessor.unwrap_or(idx);
		});
		Err(Error::NotFound)
	}

	fn leftmost(&self, mut idx: usize) -> usize {
		let mut node = &self.relation[idx];
		while let None = node.left {
			idx = node.left.unwrap();
			node = &self.relation[idx];
		}
		idx
	}

	pub fn search(&self, key: &K) -> Result<&V, Error> {
		self.find(key).map(|i| self.get_value(i).unwrap())
	}

	fn find(&self, key: &K) -> Result<usize, Error> {
		let mut i = self.root.ok_or(Error::NotFound)?;
		loop {
			let base = self.get_key(i).unwrap();
			let curr = &self.relation[i];
			let res = curr.direct(&base, key);
			match res {
				Ok(arm) => arm.map(|child_idx| i = child_idx).ok_or(Error::NotFound)?,
				Err(_) => break
			}
		}
		Ok(i)
	}
}

#[cfg(test)]
mod tests {
    use super::*;

	#[test]
	fn func() {
		let a = SizedBST::<usize, usize, 12>::from_fn(|_| Pair::new(0, 0));
	}
}