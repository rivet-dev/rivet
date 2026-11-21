use super::*;

fn parse_version(version: u64) -> Result<UrlData> {
	let url = url::Url::parse(&format!(
		"ws://localhost/?protocol_version={MIN_PROTOCOL_VERSION}&namespace=default&pool_name=default&envoy_key=envoy&version={version}"
	))
	.unwrap();
	UrlData::parse_url(url, MIN_PROTOCOL_VERSION)
}

#[test]
fn version_must_fit_reverse_index_encoding() {
	assert_eq!(
		parse_version(i32::MAX as u64).unwrap().version,
		i32::MAX as u32
	);
	assert!(parse_version(i32::MAX as u64 + 1).is_err());
	assert!(parse_version(u32::MAX as u64).is_err());
}
