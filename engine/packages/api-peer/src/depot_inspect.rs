use anyhow::{Context, Result};
use depot::{
	inspect::{self, CatalogQuery, RawScanQuery, RowsQuery, SampleQuery},
	types::{BucketId, DatabaseBranchId},
};
use rivet_api_builder::ApiCtx;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct BucketPath {
	pub bucket_id: String,
}

#[derive(Debug, Deserialize)]
pub struct DatabasePath {
	pub bucket_id: String,
	pub database_id: String,
}

#[derive(Debug, Deserialize)]
pub struct BranchPath {
	pub branch_id: String,
}

#[derive(Debug, Deserialize)]
pub struct BranchRowsPath {
	pub branch_id: String,
	pub family: String,
}

#[derive(Debug, Deserialize)]
pub struct PageTracePath {
	pub branch_id: String,
	pub pgno: u32,
}

#[derive(Debug, Deserialize)]
pub struct RawKeyPath {
	pub key: String,
}

pub async fn summary(ctx: ApiCtx, _path: (), _query: ()) -> Result<inspect::InspectResponse> {
	let udb = ctx.pools().udb()?;
	inspect::summary(&udb, ctx.pools().node_id()).await
}

pub async fn catalog(
	ctx: ApiCtx,
	_path: (),
	query: CatalogQuery,
) -> Result<inspect::CatalogResponse> {
	let udb = ctx.pools().udb()?;
	inspect::catalog(&udb, ctx.pools().node_id(), query).await
}

pub async fn bucket(
	ctx: ApiCtx,
	path: BucketPath,
	query: SampleQuery,
) -> Result<inspect::InspectResponse> {
	let bucket_id = parse_bucket_id(&path.bucket_id)?;
	let udb = ctx.pools().udb()?;
	inspect::bucket(&udb, ctx.pools().node_id(), bucket_id, query).await
}

pub async fn database(
	ctx: ApiCtx,
	path: DatabasePath,
	query: SampleQuery,
) -> Result<inspect::InspectResponse> {
	let bucket_id = parse_bucket_id(&path.bucket_id)?;
	let udb = ctx.pools().udb()?;
	inspect::database(
		&udb,
		ctx.pools().node_id(),
		bucket_id,
		path.database_id,
		query,
	)
	.await
}

pub async fn branch(
	ctx: ApiCtx,
	path: BranchPath,
	query: SampleQuery,
) -> Result<inspect::InspectResponse> {
	let branch_id = parse_database_branch_id(&path.branch_id)?;
	let udb = ctx.pools().udb()?;
	inspect::branch(&udb, ctx.pools().node_id(), branch_id, query).await
}

pub async fn page_trace(
	ctx: ApiCtx,
	path: PageTracePath,
	_query: (),
) -> Result<inspect::InspectResponse> {
	let branch_id = parse_database_branch_id(&path.branch_id)?;
	let udb = ctx.pools().udb()?;
	inspect::page_trace(&udb, ctx.pools().node_id(), branch_id, path.pgno).await
}

pub async fn branch_rows(
	ctx: ApiCtx,
	path: BranchRowsPath,
	query: RowsQuery,
) -> Result<inspect::PaginatedRowsResponse> {
	let branch_id = parse_database_branch_id(&path.branch_id)?;
	let family = inspect::RowFamily::parse(&path.family)?;
	let udb = ctx.pools().udb()?;
	inspect::branch_rows(&udb, ctx.pools().node_id(), branch_id, family, query).await
}

pub async fn raw_key(
	ctx: ApiCtx,
	path: RawKeyPath,
	_query: (),
) -> Result<inspect::InspectResponse> {
	let key = inspect::decode_path_key(&path.key)?;
	let udb = ctx.pools().udb()?;
	inspect::raw_key(&udb, ctx.pools().node_id(), key).await
}

pub async fn raw_scan(
	ctx: ApiCtx,
	_path: (),
	query: RawScanQuery,
) -> Result<inspect::PaginatedRowsResponse> {
	let udb = ctx.pools().udb()?;
	inspect::raw_scan(&udb, ctx.pools().node_id(), query).await
}

pub async fn decode_key(
	ctx: ApiCtx,
	path: RawKeyPath,
	_query: (),
) -> Result<inspect::InspectResponse> {
	let key = inspect::decode_path_key(&path.key)?;
	inspect::decode_key_response(ctx.pools().node_id(), key)
}

fn parse_bucket_id(value: &str) -> Result<BucketId> {
	Ok(BucketId::from_uuid(
		Uuid::parse_str(value).context("parse Depot bucket id")?,
	))
}

fn parse_database_branch_id(value: &str) -> Result<DatabaseBranchId> {
	Ok(DatabaseBranchId::from_uuid(
		Uuid::parse_str(value).context("parse Depot database branch id")?,
	))
}
