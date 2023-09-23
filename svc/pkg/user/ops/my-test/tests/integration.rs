use chirp_worker::prelude::*;
use redis::AsyncCommands;

#[worker_test]
async fn basic(ctx: TestCtx) {
    let mut redis = ctx.redis_cache().await.unwrap();
    redis.set::<_, _, ()>("x", 5).await.unwrap();
    let x: i64 = redis.get("x").await.unwrap();
    tracing::info!(?x);

}
