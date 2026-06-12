use xxhash_rust::xxh3::xxh3_128_with_seed;

#[test]
fn xxh3_128_with_seed_is_stable() {
	assert_eq!(
		xxh3_128_with_seed(b"rivet-envoy-test-key", 0),
		105079023209375360134079412738210681470
	);
}
