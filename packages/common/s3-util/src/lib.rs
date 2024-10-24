pub use aws_sdk_s3;

mod client;
mod provision;

pub use client::*;
pub use provision::provision;

#[derive(Clone, Debug)]
pub struct S3Bucket {
	pub name: &'static str,
}
