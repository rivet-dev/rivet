#[test]
fn action_set_requires_handler_impls() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/action_set_missing_handle.rs");
	t.compile_fail("tests/ui/queue_set_missing_handle.rs");
	t.compile_fail("tests/ui/typed_client_send_missing_handle.rs");
}
