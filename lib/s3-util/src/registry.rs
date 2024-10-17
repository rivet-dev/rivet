#[derive(Clone, Debug)]
pub struct S3Bucket {
	pub name: &'static str,
}

pub const BUCKETS: &[S3Bucket] = &[
	S3Bucket { name: "build" },
	S3Bucket { name: "cdn" },
	S3Bucket { name: "export" },
	S3Bucket { name: "banner" },
	S3Bucket { name: "logo" },
	S3Bucket { name: "artifacts" },
	S3Bucket { name: "export" },
	S3Bucket { name: "log" },
	S3Bucket {
		name: "imagor-result-storage",
	},
	S3Bucket { name: "svc-build" },
	S3Bucket {
		name: "lobby-history-export",
	},
	S3Bucket { name: "log" },
	S3Bucket { name: "avatar" },
	S3Bucket { name: "billing" },
	S3Bucket { name: "avatar" },
];
