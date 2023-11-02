#[macro_export]
macro_rules! __sql_query_telemetry {
	($ctx:expr, $action:expr) => {
		{
			let ctx = &$ctx;

			// Log query
			let location = concat!(file!(), ":", line!(), ":", column!());
			tracing::info!(%location, ty = %stringify!($rv), concat!("sql ", stringify!($action)));

			// Count metric
			rivet_pools::metrics::SQL_QUERY_TOTAL
				.with_label_values(&[stringify!($action), ctx.name(), location])
				.inc();
		}
	};
}

#[macro_export]
macro_rules! sql_query {
    ([$ctx:expr, $crdb:expr] $sql:expr, $($bind:expr),* $(,)?) => {
		{
			$crate::__sql_query_telemetry!($ctx, "execute");

			sqlx::query(indoc!($sql))
			$(
				.bind($bind)
			)*
			.execute($crdb)
		}
    };
    ([$ctx:expr] $sql:expr, $($bind:expr),* $(,)?) => {
		sql_query!([$ctx, &$ctx.crdb().await?] $sql, $($bind),*)
    };
}

#[macro_export]
macro_rules! sql_query_as {
    ([$ctx:expr, $rv:ty, $action:ident, $crdb:expr] $sql:expr, $($bind:expr),* $(,)?) => {
		{
			$crate::__sql_query_telemetry!($ctx, $action);

			sqlx::query_as::<_, $rv>(indoc!($sql))
			$(
				.bind($bind)
			)*
			.$action($crdb)
		}
    };
    ([$ctx:expr, $rv:ty, $action:ident] $sql:expr, $($bind:expr),* $(,)?) => {
		sql_query_as!([$ctx, $rv, $action, &$ctx.crdb().await?] $sql, $($bind),*)
    };
}

#[macro_export]
macro_rules! sql_fetch {
    ([$ctx:expr, $rv:ty, $crdb:expr] $sql:expr, $($bind:expr),* $(,)?) => {
		sql_query_as!([$ctx, $rv, fetch, $crdb] $sql, $($bind),*)
    };
    ([$ctx:expr, $rv:ty] $sql:expr, $($bind:expr),* $(,)?) => {
		sql_query_as!([$ctx, $rv, fetch] $sql, $($bind),*)
    };
}

#[macro_export]
macro_rules! sql_fetch_all {
    ([$ctx:expr, $rv:ty, $crdb:expr] $sql:expr, $($bind:expr),* $(,)?) => {
		sql_query_as!([$ctx, $rv, fetch_all, $crdb] $sql, $($bind),*)
    };
    ([$ctx:expr, $rv:ty] $sql:expr, $($bind:expr),* $(,)?) => {
		sql_query_as!([$ctx, $rv, fetch_all] $sql, $($bind),*)
    };
}

#[macro_export]
macro_rules! sql_fetch_many {
    ([$ctx:expr, $rv:ty, $crdb:expr] $sql:expr, $($bind:expr),* $(,)?) => {
		sql_query_as!([$ctx, $rv, fetch_many, $crdb] $sql, $($bind),*)
    };
    ([$ctx:expr, $rv:ty] $sql:expr, $($bind:expr),* $(,)?) => {
		sql_query_as!([$ctx, $rv, fetch_many] $sql, $($bind),*)
    };
}

#[macro_export]
macro_rules! sql_fetch_one {
    ([$ctx:expr, $rv:ty, $crdb:expr] $sql:expr, $($bind:expr),* $(,)?) => {
		sql_query_as!([$ctx, $rv, fetch_one, $crdb] $sql, $($bind),*)
    };
    ([$ctx:expr, $rv:ty] $sql:expr, $($bind:expr),* $(,)?) => {
		sql_query_as!([$ctx, $rv, fetch_one] $sql, $($bind),*)
    };
}

#[macro_export]
macro_rules! sql_fetch_optional {
    ([$ctx:expr, $rv:ty, $crdb:expr] $sql:expr, $($bind:expr),* $(,)?) => {
		sql_query_as!([$ctx, $rv, fetch_optional, $crdb] $sql, $($bind),*)
    };
    ([$ctx:expr, $rv:ty] $sql:expr, $($bind:expr),* $(,)?) => {
		sql_query_as!([$ctx, $rv, fetch_optional] $sql, $($bind),*)
    };
}
