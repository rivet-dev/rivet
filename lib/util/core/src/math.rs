use std::cmp::Ordering;

fn _cmp_floats(a: f32, b: f32) -> Ordering {
	a.partial_cmp(&b).unwrap_or_else(|| {
		if a.is_nan() {
			if b.is_nan() {
				Ordering::Equal
			} else {
				Ordering::Less
			}
		} else {
			Ordering::Greater
		}
	})
}

fn _cmp_floats_opt(a: Option<f32>, b: Option<f32>) -> Ordering {
	match a.partial_cmp(&b) {
		Some(ord) => ord,
		None => {
			if let (Some(a), Some(b)) = (a, b) {
				if a.is_nan() {
					if b.is_nan() {
						Ordering::Equal
					} else {
						Ordering::Less
					}
				} else if b.is_nan() {
					Ordering::Greater
				} else {
					// unreachable
					Ordering::Less
				}
			} else {
				// unreachable
				Ordering::Less
			}
		}
	}
}

/// Divide integers of any type, rounding up. Panics on dividing by 0.
#[macro_export]
macro_rules! div_up {
	($a:expr, $b:expr) => {
		($a + ($b - 1)) / $b
	};
}
