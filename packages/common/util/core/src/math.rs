/// Divide integers of any type, rounding up. Panics on dividing by 0.
#[macro_export]
macro_rules! div_up {
	($a:expr, $b:expr) => {
		($a + ($b - 1)) / $b
	};
}
