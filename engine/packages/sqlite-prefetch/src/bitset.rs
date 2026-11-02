//! A growable set of page numbers, one bit per page.
use crate::page::Pgno;

#[derive(Debug, Default, Clone)]
pub(crate) struct PageBits {
	words: std::collections::BTreeMap<usize, u64>,
}

impl PageBits {
	pub fn get(&self, pgno: Pgno) -> bool {
		let (word, bit) = Self::locate(pgno);
		self.words.get(&word).is_some_and(|w| w & (1 << bit) != 0)
	}

	pub fn set(&mut self, pgno: Pgno) -> bool {
		let (word, bit) = Self::locate(pgno);
		let mut evicted = false;
		// Knowledge of absence is optional. Bound it even when a corrupt page names an
		// enormous page number; forgetting an old word only requires another lookup.
		if !self.words.contains_key(&word) && self.words.len() == 4096 {
			self.words.pop_first();
			evicted = true;
		}
		*self.words.entry(word).or_default() |= 1 << bit;
		evicted
	}

	pub fn clear(&mut self, pgno: Pgno) {
		let (word, bit) = Self::locate(pgno);
		if let Some(w) = self.words.get_mut(&word) {
			*w &= !(1 << bit);
		}
	}

	/// Clears every page above `size_pages`, so the set never outgrows the file.
	pub fn truncate(&mut self, size_pages: u32) {
		let keep = (size_pages as usize).div_ceil(64);
		self.words.retain(|word, _| *word < keep);
		let used = size_pages as usize % 64;
		if used != 0
			&& let Some(last) = self.words.get_mut(&keep.saturating_sub(1))
		{
			*last &= (1u64 << used) - 1;
		}
	}

	pub fn reset(&mut self) {
		self.words.clear();
	}

	pub fn bytes(&self) -> usize {
		self.words.len() * 32
	}

	/// Page `pgno` is bit `pgno - 1`.
	fn locate(pgno: Pgno) -> (usize, u32) {
		let index = pgno.saturating_sub(1) as usize;
		(index / 64, (index % 64) as u32)
	}
}
