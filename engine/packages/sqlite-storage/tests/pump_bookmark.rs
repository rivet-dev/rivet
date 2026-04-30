use sqlite_storage::pump::types::BookmarkStr;

#[test]
fn bookmark_format_is_fixed_width_hex() {
	let bookmark = BookmarkStr::format(1_700_000_000_000, 42).expect("bookmark should format");

	assert_eq!(bookmark.as_str(), "0000018bcfe56800-000000000000002a");
	assert_eq!(
		bookmark.parse().expect("bookmark should parse"),
		(1_700_000_000_000, 42)
	);
}

#[test]
fn bookmark_new_rejects_malformed_wire_strings() {
	let cases = [
		"",
		"0000018bcfe56800",
		"0000018bcfe56800_000000000000002a",
		"0000018bcfe5680-000000000000002a",
		"0000018bcfe56800-00000000000002ag",
		"0000018bcfe56800-000000000000002a00",
		"0000018bcfe56800-00000000000002🙂",
	];

	for case in cases {
		assert!(BookmarkStr::new(case).is_err(), "{case} should be rejected");
	}
}

#[test]
fn bookmark_format_rejects_negative_timestamps() {
	assert!(BookmarkStr::format(-1, 0).is_err());
}

#[test]
fn bookmark_round_trip_property_for_representative_values() {
	let timestamps = [
		0,
		1,
		999,
		1_700_000_000_000,
		i64::MAX / 2,
		i64::MAX,
	];
	let txids = [0, 1, 42, u32::MAX as u64, u64::MAX - 1, u64::MAX];

	for ts_ms in timestamps {
		for txid in txids {
			let bookmark = BookmarkStr::format(ts_ms, txid).expect("bookmark should format");
			assert_eq!(bookmark.as_str().len(), 33);
			assert_eq!(
				bookmark.parse().expect("bookmark should parse"),
				(ts_ms, txid)
			);
		}
	}
}

#[test]
fn bookmark_lex_order_matches_chronological_order_for_one_branch() {
	let mut bookmarks = vec![
		BookmarkStr::format(10, 5).expect("bookmark should format"),
		BookmarkStr::format(9, u64::MAX).expect("bookmark should format"),
		BookmarkStr::format(10, 4).expect("bookmark should format"),
		BookmarkStr::format(11, 0).expect("bookmark should format"),
	];

	bookmarks.sort();

	let parsed = bookmarks
		.into_iter()
		.map(|bookmark| bookmark.parse().expect("bookmark should parse"))
		.collect::<Vec<_>>();

	assert_eq!(parsed, vec![(9, u64::MAX), (10, 4), (10, 5), (11, 0)]);
}
