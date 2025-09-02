use serde::{Deserialize, Serialize};
use std::ops::Deref;

/// Represents the location of an event in history.
#[derive(Debug, Clone, Hash, PartialOrd, Ord, PartialEq, Eq, Serialize, Deserialize)]
pub struct Location(Box<[Coordinate]>);

impl Location {
	pub fn new(raw: Box<[Coordinate]>) -> Self {
		Location(raw)
	}

	pub fn empty() -> Self {
		Location(Box::new([]))
	}

	pub fn root(&self) -> Location {
		self.0
			.iter()
			.take(self.0.len().saturating_sub(1))
			.cloned()
			.collect()
	}

	pub fn tail(&self) -> Option<&Coordinate> {
		self.0.last()
	}

	pub fn join(&self, coordinate: Coordinate) -> Location {
		self.0
			.iter()
			.cloned()
			.chain(std::iter::once(coordinate))
			.collect()
	}
}

impl std::fmt::Display for Location {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{{")?;

		for (i, coord) in self.0.iter().enumerate() {
			write!(f, "{}", coord)?;

			if i != self.0.len() - 1 {
				write!(f, ", ")?;
			}
		}

		write!(f, "}}")
	}
}

impl Deref for Location {
	type Target = [Coordinate];

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl IntoIterator for Location {
	type Item = Coordinate;
	type IntoIter = std::vec::IntoIter<Coordinate>;

	fn into_iter(self) -> Self::IntoIter {
		self.0.into_vec().into_iter()
	}
}

impl FromIterator<Vec<usize>> for Location {
	fn from_iter<I: IntoIterator<Item = Vec<usize>>>(iter: I) -> Self {
		Location(
			iter.into_iter()
				.map(|v| Coordinate::new(v.into_boxed_slice()))
				.collect(),
		)
	}
}

impl FromIterator<Box<[usize]>> for Location {
	fn from_iter<I: IntoIterator<Item = Box<[usize]>>>(iter: I) -> Self {
		Location(iter.into_iter().map(Coordinate::new).collect())
	}
}

impl FromIterator<Coordinate> for Location {
	fn from_iter<I: IntoIterator<Item = Coordinate>>(iter: I) -> Self {
		Location(iter.into_iter().collect())
	}
}

/// Represents a position within a location.
#[derive(Debug, Clone, Hash, PartialOrd, Ord, PartialEq, Eq, Serialize, Deserialize)]
pub struct Coordinate(Box<[usize]>);

impl Coordinate {
	pub fn new(raw: Box<[usize]>) -> Self {
		Coordinate(raw)
	}

	pub fn simple(int: usize) -> Self {
		Coordinate(Box::new([int]))
	}

	pub fn with_tail(&self, tail: usize) -> Self {
		self.0
			.iter()
			.take(self.0.len().saturating_sub(1))
			.cloned()
			.chain(std::iter::once(tail))
			.collect()
	}

	pub fn head(&self) -> usize {
		*self.0.first().expect("empty coordinate")
	}

	pub fn tail(&self) -> usize {
		*self.0.last().expect("empty coordinate")
	}
}

impl std::fmt::Display for Coordinate {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		for (i, x) in self.0.iter().enumerate() {
			write!(f, "{}", x)?;

			if i != self.0.len() - 1 {
				write!(f, ".")?;
			}
		}

		Ok(())
	}
}

impl IntoIterator for Coordinate {
	type Item = usize;
	type IntoIter = std::vec::IntoIter<usize>;

	fn into_iter(self) -> Self::IntoIter {
		self.0.into_vec().into_iter()
	}
}

impl FromIterator<usize> for Coordinate {
	fn from_iter<I: IntoIterator<Item = usize>>(iter: I) -> Self {
		Coordinate(iter.into_iter().collect())
	}
}

impl Deref for Coordinate {
	type Target = [usize];

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

mod fdb {
	use super::Coordinate;
	use udb_util::prelude::*;

	impl TuplePack for Coordinate {
		fn pack<W: std::io::Write>(
			&self,
			w: &mut W,
			tuple_depth: TupleDepth,
		) -> std::io::Result<VersionstampOffset> {
			// Pack the vector of usize values directly
			self.0.as_ref().pack(w, tuple_depth.increment())
		}
	}

	impl<'de> TupleUnpack<'de> for Coordinate {
		fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
			// Unpack as a vector of usize values, then convert to Coordinate
			let (input, values) = <Vec<usize>>::unpack(input, tuple_depth.increment())?;
			let coordinate = Coordinate::new(values.into_boxed_slice());

			Ok((input, coordinate))
		}
	}
}
