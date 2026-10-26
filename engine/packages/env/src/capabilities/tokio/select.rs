//! Reimplements `tokio::select!` with an explicit RNG for reproducible branch
//! selection. `select_biased!` provides source-order polling with a required reason.

#[doc(hidden)]
pub enum Choice<A, B, C> {
	First(A),
	Second(B),
	Third(C),
}

/// Fair selection of two or three futures using the environment's labelled RNG.
/// Branch bodies run only after selection; losing futures are dropped first.
/// Supports irrefutable patterns and two or three branches, without guards or `else`.
/// Handle `Option`/`Result` inside the branch body. For conditional work, construct
/// a future that waits indefinitely while disabled. Extend this subset only when
/// a real caller requires it; this is not a drop-in replacement for Tokio's macro.
#[macro_export]
macro_rules! select {
    ($rng:expr; $a:pat = $af:expr => $ab:expr, $b:pat = $bf:expr => $bb:expr $(,)?) => {
        $crate::__select_poll!($rng.gen_range(0usize..2); $a = $af => $ab, $b = $bf => $bb, _ = std::future::pending::<()>() => unreachable!())
    };
    ($rng:expr; $a:pat = $af:expr => $ab:expr, $b:pat = $bf:expr => $bb:expr, $c:pat = $cf:expr => $cb:expr $(,)?) => {
        $crate::__select_poll!($rng.gen_range(0usize..3); $a = $af => $ab, $b = $bf => $bb, $c = $cf => $cb)
    };
}

/// Source-order selection. The reason explains why ordering is required.
#[macro_export]
macro_rules! select_biased {
    ($reason:literal, $a:pat = $af:expr => $ab:expr, $b:pat = $bf:expr => $bb:expr $(,)?) => {
        $crate::__select_poll!(0usize; $a = $af => $ab, $b = $bf => $bb, _ = std::future::pending::<()>() => unreachable!())
    };
    ($reason:literal, $a:pat = $af:expr => $ab:expr, $b:pat = $bf:expr => $bb:expr, $c:pat = $cf:expr => $cb:expr $(,)?) => {
        $crate::__select_poll!(0usize; $a = $af => $ab, $b = $bf => $bb, $c = $cf => $cb)
    };
    () => { compile_error!("select_biased! requires a reason string") };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __select_poll {
	($first:expr; $a:pat = $af:expr => $ab:expr, $b:pat = $bf:expr => $bb:expr, $c:pat = $cf:expr => $cb:expr) => {{
		let selected = {
			let first = $first;
			let mut a = std::pin::pin!($af);
			let mut b = std::pin::pin!($bf);
			let mut c = std::pin::pin!($cf);
			std::future::poll_fn(|cx| {
				use std::{future::Future, task::Poll};

				for offset in 0..3 {
					match (first + offset) % 3 {
						0 => {
							if let Poll::Ready(v) = a.as_mut().poll(cx) {
								return Poll::Ready($crate::__Choice::First(v));
							}
						}
						1 => {
							if let Poll::Ready(v) = b.as_mut().poll(cx) {
								return Poll::Ready($crate::__Choice::Second(v));
							}
						}
						_ => {
							if let Poll::Ready(v) = c.as_mut().poll(cx) {
								return Poll::Ready($crate::__Choice::Third(v));
							}
						}
					}
				}

				Poll::Pending
			})
			.await
		};

		match selected {
			$crate::__Choice::First($a) => $ab,
			$crate::__Choice::Second($b) => $bb,
			$crate::__Choice::Third($c) => $cb,
		}
	}};
}

/// Disabled selection branches evaluate their future but never poll it.
pub async fn when<F: std::future::Future>(enabled: bool, future: F) -> F::Output {
	if enabled {
		future.await
	} else {
		std::future::pending().await
	}
}
