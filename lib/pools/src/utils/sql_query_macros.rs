#[macro_export]
macro_rules! sql_query {
    ([$ctx:expr, $crdb:expr] $sql:expr, $($bind:expr),*) => {
        sqlx::query(indoc!($sql))
        $(
            .bind($bind)
        )*
        .execute($crdb)
        .await
    };
    ([$ctx:expr] $sql:expr, $($bind:expr),*) => {
		let crdb = $ctx.crdb().await?;
		sql_query!([ctx, &crdb] $sql, $($bind),*).await
    };
}

#[macro_export]
macro_rules! sql_query_as {
    ([$ctx:expr, $rv:ty, $action:ident, $crdb:expr] $sql:expr, $($bind:expr),*) => {
        sqlx::query_as::<_, $rv>(indoc!($sql))
        $(
            .bind($bind)
        )*
        .$action($crdb)
        .await
    };
    ([$ctx:expr, $rv:ty, $action:ident] $sql:expr, $($bind:expr),*) => {
		{
			let crdb = $ctx.crdb().await?;
			sql_query_as!([ctx, $rv, $action, &crdb] $sql, $($bind),*)
		}
    };
}

#[macro_export]
macro_rules! sql_fetch {
    ([$ctx:expr, $rv:ty, $crdb:ident] $sql:expr, $($bind:expr),*) => {
		sql_query_as!([$ctx, $rv, fetch, $crdb] $sql, $($bind),*)
    };
    ([$ctx:expr, $rv:ty, $action:ident] $sql:expr, $($bind:expr),*) => {
		sql_query_as!([$ctx, $rv, fetch] $sql, $($bind),*)
    };
}

#[macro_export]
macro_rules! sql_fetch_all {
    ([$ctx:expr, $rv:ty, $crdb:ident] $sql:expr, $($bind:expr),*) => {
		sql_query_as!([$ctx, $rv, fetch_all, $crdb] $sql, $($bind),*)
    };
    ([$ctx:expr, $rv:ty] $sql:expr, $($bind:expr),*) => {
		sql_query_as!([$ctx, $rv, fetch_all] $sql, $($bind),*)
    };
}

#[macro_export]
macro_rules! sql_fetch_many {
    ([$ctx:expr, $rv:ty, $crdb:ident] $sql:expr, $($bind:expr),*) => {
		sql_query_as!([$ctx, $rv, fetch_many, $crdb] $sql, $($bind),*)
    };
    ([$ctx:expr, $rv:ty] $sql:expr, $($bind:expr),*) => {
		sql_query_as!([$ctx, $rv, fetch_many] $sql, $($bind),*)
    };
}

#[macro_export]
macro_rules! sql_fetch_one {
    ([$ctx:expr, $rv:ty, $crdb:ident] $sql:expr, $($bind:expr),*) => {
		sql_query_as!([$ctx, $rv, fetch_one, $crdb] $sql, $($bind),*)
    };
    ([$ctx:expr, $rv:ty] $sql:expr, $($bind:expr),*) => {
		sql_query_as!([$ctx, $rv, fetch_one] $sql, $($bind),*)
    };
}

#[macro_export]
macro_rules! sql_fetch_optional {
    ([$ctx:expr, $rv:ty, $crdb:ident] $sql:expr, $($bind:expr),*) => {
		sql_query_as!([$ctx, $rv, fetch_optional, $crdb] $sql, $($bind),*)
    };
    ([$ctx:expr, $rv:ty] $sql:expr, $($bind:expr),*) => {
		sql_query_as!([$ctx, $rv, fetch_optional] $sql, $($bind),*)
    };
}
