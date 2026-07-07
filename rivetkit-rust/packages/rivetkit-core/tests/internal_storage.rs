use super::*;

#[test]
fn required_blob_null_decodes_as_empty_blob() {
	let row = vec![ColumnValue::Null];
	assert_eq!(read_blob(&row, 0, "state").unwrap(), Vec::<u8>::new());
}
