use rivetkit::test;
use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
async fn counter_actor_works_through_rust_client() -> anyhow::Result<()> {
	let h = test::setup(example_hello_world_rust::registry()).await?;
	let counter = h.actor(example_hello_world_rust::ACTOR_NAME);

	assert_eq!(counter.action("increment", vec![json!(1)]).await?, json!(1));
	assert_eq!(counter.action("increment", vec![json!(1)]).await?, json!(2));
	assert_eq!(counter.action("getCount", vec![]).await?, json!(2));

	h.shutdown().await;
	Ok(())
}
