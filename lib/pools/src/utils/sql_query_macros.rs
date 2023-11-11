lazy_static::lazy_static! {
	/// Rate limit used to limit creating a stampede of connections to the database.
	pub static ref CONN_ACQUIRE_RATE_LIMIT: governor::RateLimiter<
		governor::state::direct::NotKeyed,
		governor::state::InMemoryState,
		governor::clock::DefaultClock,
		governor::middleware::NoOpMiddleware
	> = governor::RateLimiter::direct(
		// Limit how many connections can be created to the database
		governor::Quota::per_second(std::num::NonZeroU32::new(10).unwrap())
			// Allow creating at most 5 connections at the same time
			.allow_burst(std::num::NonZeroU32::new(5).unwrap())
	);
}

// MARK: Metrics
#[macro_export]
macro_rules! __sql_query_metrics_acquire {
	($acquire_timer:ident) => {
		// Start timer
		let $acquire_timer = tokio::time::Instant::now();
	};
}

#[macro_export]
macro_rules! __sql_query_metrics_start {
	($ctx:expr, $action:expr, $acquire_timer:ident, $start_timer:ident) => {{
		let ctx = &$ctx;
		let location = concat!(file!(), ":", line!(), ":", column!());

		// Count acquire
		let acquire_duration = $acquire_timer.elapsed().as_secs_f64();
		rivet_pools::metrics::SQL_ACQUIRE_DURATION
			.with_label_values(&[stringify!($action), ctx.name(), location])
			.observe(acquire_duration);

		// Count metric
		rivet_pools::metrics::SQL_QUERY_TOTAL
			.with_label_values(&[stringify!($action), ctx.name(), location])
			.inc();
	}

	// Start timer
	let $start_timer = tokio::time::Instant::now();};
}

#[macro_export]
macro_rules! __sql_query_metrics_finish {
	($ctx:expr, $action:expr, $start_timer:ident) => {{
		let ctx = &$ctx;

		let duration = $start_timer.elapsed().as_secs_f64();

		// Log query
		let location = concat!(file!(), ":", line!(), ":", column!());
		tracing::info!(%location, ty = %stringify!($rv), dt = ?duration, action = stringify!($action), "sql query");

		// Count metric
		rivet_pools::metrics::SQL_QUERY_DURATION
			.with_label_values(&[stringify!($action), ctx.name(), location])
			.observe(duration);
	}};
}

// MARK: Helpers
/// Acquire a connection from the pool with a rate limiting mechanism to reuse existing connecting
/// if possible.
///
/// First, attempt to acquire a connection with `.try_acquire()` if one already exists to use.
///
///	If none are available, check the rate limit to see if we can create a new connection. If so,
///	call `.acquire()` which will create a new connection in the pool.
///
///	If the rate limit is exhausted, this will sleep and try again.
///
///	This is in order to prevent connection spikes that will create a lot of connections in
///	parallel, adding strain to the database & slowing down the query.
///
///	Without this, initial queries are very very slow because it's slower to create 32 new connections
///	than make 4 RTT queries over 8 existing connections.
#[macro_export]
macro_rules! __sql_acquire {
	($ctx:expr, $crdb:expr) => {{
		let location = concat!(file!(), ":", line!(), ":", column!());

		let mut tries = 0;
		let (conn, acquire_result) = loop {
			tries += 1;

			// Attempt to use an existing connection
			if let Some(conn) = $crdb.try_acquire() {
				break (conn, "try_acquire");
			} else {
				// Check if we can create a new connection
				if $crate::utils::sql_query_macros::CONN_ACQUIRE_RATE_LIMIT
					.check()
					.is_ok()
				{
					// Create a new connection
					break ($crdb.acquire().await?, "acquire");
				} else {
					// TODO: Backoff
					tokio::time::sleep(std::time::Duration::from_millis(1)).await;
				}
			}
		};

		rivet_pools::metrics::SQL_ACQUIRE_TOTAL
			.with_label_values(&[stringify!($action), $ctx.name(), location, acquire_result])
			.inc();
		rivet_pools::metrics::SQL_ACQUIRE_TRIES
			.with_label_values(&[stringify!($action), $ctx.name(), location, acquire_result])
			.inc_by(tries);

		conn
	}};
}

#[macro_export]
macro_rules! __sql_query {
    ([$ctx:expr, $crdb:expr] $sql:expr, $($bind:expr),* $(,)?) => {
		async {
			use sqlx::Acquire;

			let query = sqlx::query(indoc!($sql))
			$(
				.bind($bind)
			)*;

			// Acquire connection
			$crate::__sql_query_metrics_acquire!(_acquire);
			let crdb = $crdb;
			let mut conn = $crate::__sql_acquire!($ctx, crdb);

			// Execute query
			$crate::__sql_query_metrics_start!($ctx, execute, _acquire, _start);
			let res = query.execute(&mut *conn).await.map_err(Into::<GlobalError>::into);
			$crate::__sql_query_metrics_finish!($ctx, execute, _start);

			res
		}
    };
    ([$ctx:expr, @tx $tx:expr] $sql:expr, $($bind:expr),* $(,)?) => {
		async {
			let query = sqlx::query(indoc!($sql))
			$(
				.bind($bind)
			)*;

			// Execute query
			$crate::__sql_query_metrics_acquire!(_acquire);
			$crate::__sql_query_metrics_start!($ctx, execute, _acquire, _start);
			let res = query.execute(&mut **$tx).await.map_err(Into::<GlobalError>::into);
			$crate::__sql_query_metrics_finish!($ctx, execute, _start);

			res
		}
    };
    ([$ctx:expr] $sql:expr, $($bind:expr),* $(,)?) => {
		__sql_query!([$ctx, &$ctx.crdb().await?] $sql, $($bind),*)
    };
}

#[macro_export]
macro_rules! __sql_query_as {
    ([$ctx:expr, $rv:ty, $action:ident, $crdb:expr] $sql:expr, $($bind:expr),* $(,)?) => {
		async {
			use sqlx::Acquire;

			let query = sqlx::query_as::<_, $rv>(indoc!($sql))
			$(
				.bind($bind)
			)*;

			// Acquire connection
			$crate::__sql_query_metrics_acquire!(_acquire);
			let crdb = $crdb;
			let mut conn = $crate::__sql_acquire!($ctx, crdb);

			// Execute query
			$crate::__sql_query_metrics_start!($ctx, $action, _acquire, _start);
			let res = query.$action(&mut *conn).await.map_err(Into::<GlobalError>::into);
			$crate::__sql_query_metrics_finish!($ctx, $action, _start);

			res
		}
    };
    ([$ctx:expr, $rv:ty, $action:ident, @tx $tx:expr] $sql:expr, $($bind:expr),* $(,)?) => {
		async {
			let query = sqlx::query_as::<_, $rv>(indoc!($sql))
			$(
				.bind($bind)
			)*;

			// Execute query
			$crate::__sql_query_metrics_acquire!(_acquire);
			$crate::__sql_query_metrics_start!($ctx, $action, _acquire, _start);
			let res = query.$action(&mut **$tx).await.map_err(Into::<GlobalError>::into);
			$crate::__sql_query_metrics_finish!($ctx, $action, _start);

			res
		}
    };
    ([$ctx:expr, $rv:ty, $action:ident] $sql:expr, $($bind:expr),* $(,)?) => {
		__sql_query_as!([$ctx, $rv, $action, &$ctx.crdb().await?] $sql, $($bind),*)
    };
}

/// Returns a query without being wrapped in an async block, and therefore cannot time the query.
/// Used for the `fetch` function.
#[macro_export]
macro_rules! __sql_query_as_raw {
    ([$ctx:expr, $rv:ty, $action:ident, $crdb:expr] $sql:expr, $($bind:expr),* $(,)?) => {
		// We can't record metrics for this because we can't move the `await` in to this macro
		sqlx::query_as::<_, $rv>(indoc!($sql))
		$(
			.bind($bind)
		)*
		.$action($crdb)
    };
    ([$ctx:expr, $rv:ty, $action:ident] $sql:expr, $($bind:expr),* $(,)?) => {
		__sql_query_as!([$ctx, $rv, $action, &$ctx.crdb().await?] $sql, $($bind),*)
    };
}

// MARK: Specific actions
#[macro_export]
macro_rules! sql_execute {
    ($($arg:tt)*) => {
		__sql_query!($($arg)*)
    };
}

#[macro_export]
macro_rules! sql_fetch {
    ([$ctx:expr, $rv:ty, $crdb:expr] $sql:expr, $($bind:expr),* $(,)?) => {
		__sql_query_as_raw!([$ctx, $rv, fetch, $crdb] $sql, $($bind),*)
    };
    ([$ctx:expr, $rv:ty, @tx $crdb:expr] $sql:expr, $($bind:expr),* $(,)?) => {
		__sql_query_as_raw!([$ctx, $rv, fetch, @tx $crdb] $sql, $($bind),*)
    };
    ([$ctx:expr, $rv:ty] $sql:expr, $($bind:expr),* $(,)?) => {
		__sql_query_as_raw!([$ctx, $rv, fetch] $sql, $($bind),*)
    };
}

#[macro_export]
macro_rules! sql_fetch_all {
    ([$ctx:expr, $rv:ty, $crdb:expr] $sql:expr, $($bind:expr),* $(,)?) => {
		__sql_query_as!([$ctx, $rv, fetch_all, $crdb] $sql, $($bind),*)
    };
    ([$ctx:expr, $rv:ty, @tx $crdb:expr] $sql:expr, $($bind:expr),* $(,)?) => {
		__sql_query_as!([$ctx, $rv, fetch_all, @tx $crdb] $sql, $($bind),*)
    };
    ([$ctx:expr, $rv:ty] $sql:expr, $($bind:expr),* $(,)?) => {
		__sql_query_as!([$ctx, $rv, fetch_all] $sql, $($bind),*)
    };
}

#[macro_export]
macro_rules! sql_fetch_many {
    ([$ctx:expr, $rv:ty, $crdb:expr] $sql:expr, $($bind:expr),* $(,)?) => {
		__sql_query_as!([$ctx, $rv, fetch_many, $crdb] $sql, $($bind),*)
    };
    ([$ctx:expr, $rv:ty, @tx $crdb:expr] $sql:expr, $($bind:expr),* $(,)?) => {
		__sql_query_as!([$ctx, $rv, fetch_many, @tx $crdb] $sql, $($bind),*)
    };
    ([$ctx:expr, $rv:ty] $sql:expr, $($bind:expr),* $(,)?) => {
		__sql_query_as!([$ctx, $rv, fetch_many] $sql, $($bind),*)
    };
}

#[macro_export]
macro_rules! sql_fetch_one {
    ([$ctx:expr, $rv:ty, $crdb:expr] $sql:expr, $($bind:expr),* $(,)?) => {
		__sql_query_as!([$ctx, $rv, fetch_one, $crdb] $sql, $($bind),*)
    };
    ([$ctx:expr, $rv:ty, @tx $crdb:expr] $sql:expr, $($bind:expr),* $(,)?) => {
		__sql_query_as!([$ctx, $rv, fetch_one, @tx $crdb] $sql, $($bind),*)
    };
    ([$ctx:expr, $rv:ty] $sql:expr, $($bind:expr),* $(,)?) => {
		__sql_query_as!([$ctx, $rv, fetch_one] $sql, $($bind),*)
    };
}

#[macro_export]
macro_rules! sql_fetch_optional {
    ([$ctx:expr, $rv:ty, $crdb:expr] $sql:expr, $($bind:expr),* $(,)?) => {
		__sql_query_as!([$ctx, $rv, fetch_optional, $crdb] $sql, $($bind),*)
    };
    ([$ctx:expr, $rv:ty, @tx $crdb:expr] $sql:expr, $($bind:expr),* $(,)?) => {
		__sql_query_as!([$ctx, $rv, fetch_optional, @tx $crdb] $sql, $($bind),*)
    };
    ([$ctx:expr, $rv:ty] $sql:expr, $($bind:expr),* $(,)?) => {
		__sql_query_as!([$ctx, $rv, fetch_optional] $sql, $($bind),*)
    };
}
