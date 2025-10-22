/// Divide integers of any type, rounding up. Panics on dividing by 0.
#[deprecated(note = "use x.div_ceil(y) instead")]
#[macro_export]
macro_rules! div_up {
	($a:expr, $b:expr) => {
		($a + ($b - 1)) / $b
	};
}

/// Performs ceiling division for i64 values.
///
/// Returns the smallest integer greater than or equal to `a / b`.
///
/// # Examples
/// ```
/// assert_eq!(div_ceil_i64(10, 3), 4);  // 10/3 = 3.33.. -> 4
/// assert_eq!(div_ceil_i64(9, 3), 3);   // 9/3 = 3 -> 3
/// assert_eq!(div_ceil_i64(-10, 3), -3); // -10/3 = -3.33.. -> -3
/// ```
///
/// # Panics
/// Panics if `b` is zero.
pub fn div_ceil_i64(a: i64, b: i64) -> i64 {
	if b == 0 {
		panic!("attempt to divide by zero");
	}

	if a == 0 || (a > 0 && b > 0) || (a < 0 && b < 0) {
		// Standard ceiling division when signs match or a is zero
		(a + b - 1) / b
	} else {
		// When signs differ, regular division gives the ceiling
		a / b
	}
}
