//! Reimplements `tokio::select!` with an explicit RNG for reproducible branch
//! selection. `select_biased!` provides source-order polling with a required reason.

#[doc(hidden)]
pub enum Choice<A, B, C, D> {
	First(A),
	Second(B),
	Third(C),
	Fourth(D),
}

/// Fair selection of two to four futures using the environment's labelled RNG.
/// Branch bodies run only after selection; losing futures are dropped first.
/// Supports irrefutable patterns and two to four branches, without guards or `else`.
/// Handle `Option`/`Result` inside the branch body. For conditional work, construct
/// a future that waits indefinitely while disabled. Extend this subset only when
/// a real caller requires it; this is not a drop-in replacement for Tokio's macro.
#[macro_export]
macro_rules! select {
    ($rng:expr; $a:pat = $af:expr => $ab:expr, $b:pat = $bf:expr => $bb:expr $(,)?) => {
        $crate::__select_poll!($rng.gen_range(0usize..2), 2; $a = $af => $ab, $b = $bf => $bb, _ = std::future::pending::<()>() => unreachable!(), _ = std::future::pending::<()>() => unreachable!())
    };
    ($rng:expr; $a:pat = $af:expr => $ab:expr, $b:pat = $bf:expr => $bb:expr, $c:pat = $cf:expr => $cb:expr $(,)?) => {
        $crate::__select_poll!($rng.gen_range(0usize..3), 3; $a = $af => $ab, $b = $bf => $bb, $c = $cf => $cb, _ = std::future::pending::<()>() => unreachable!())
    };
    ($rng:expr; $a:pat = $af:expr => $ab:expr, $b:pat = $bf:expr => $bb:expr, $c:pat = $cf:expr => $cb:expr, $d:pat = $df:expr => $db:expr $(,)?) => {
        $crate::__select_poll!($rng.gen_range(0usize..4), 4; $a = $af => $ab, $b = $bf => $bb, $c = $cf => $cb, $d = $df => $db)
    };
}

/// Source-order selection of two to four futures. The reason explains why ordering is required.
#[macro_export]
macro_rules! select_biased {
    ($reason:literal, $a:pat = $af:expr => $ab:expr, $b:pat = $bf:expr => $bb:expr $(,)?) => {
        $crate::__select_poll!(0usize, 2; $a = $af => $ab, $b = $bf => $bb, _ = std::future::pending::<()>() => unreachable!(), _ = std::future::pending::<()>() => unreachable!())
    };
    ($reason:literal, $a:pat = $af:expr => $ab:expr, $b:pat = $bf:expr => $bb:expr, $c:pat = $cf:expr => $cb:expr $(,)?) => {
        $crate::__select_poll!(0usize, 3; $a = $af => $ab, $b = $bf => $bb, $c = $cf => $cb, _ = std::future::pending::<()>() => unreachable!())
    };
    ($reason:literal, $a:pat = $af:expr => $ab:expr, $b:pat = $bf:expr => $bb:expr, $c:pat = $cf:expr => $cb:expr, $d:pat = $df:expr => $db:expr $(,)?) => {
        $crate::__select_poll!(0usize, 4; $a = $af => $ab, $b = $bf => $bb, $c = $cf => $cb, $d = $df => $db)
    };
    () => { compile_error!("select_biased! requires a reason string") };
}

/// Polls the first `$count` branches in rotation from `$first`. Callers with fewer than four
/// branches pad with never-ready futures past `$count`, which are never polled.
#[doc(hidden)]
#[macro_export]
macro_rules! __select_poll {
	($first:expr, $count:literal; $a:pat = $af:expr => $ab:expr, $b:pat = $bf:expr => $bb:expr, $c:pat = $cf:expr => $cb:expr, $d:pat = $df:expr => $db:expr) => {{
		let selected = {
			let first = $first;
			let mut a = std::pin::pin!($af);
			let mut b = std::pin::pin!($bf);
			let mut c = std::pin::pin!($cf);
			let mut d = std::pin::pin!($df);
			std::future::poll_fn(|cx| {
				use std::{future::Future, task::Poll};

				for offset in 0..$count {
					match (first + offset) % $count {
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
						2 => {
							if let Poll::Ready(v) = c.as_mut().poll(cx) {
								return Poll::Ready($crate::__Choice::Third(v));
							}
						}
						_ => {
							if let Poll::Ready(v) = d.as_mut().poll(cx) {
								return Poll::Ready($crate::__Choice::Fourth(v));
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
			$crate::__Choice::Fourth($d) => $db,
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
