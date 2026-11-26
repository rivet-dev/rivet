package dev.rivet.examples;

import dev.rivet.kit.Actor;
import dev.rivet.kit.ActorContext;
import dev.rivet.kit.Name;
import dev.rivet.kit.None;
import dev.rivet.kit.Registry;

/**
 * Java side of the SDK comparison benchmark ({@code mise run bench:compare}): a
 * {@code counter} actor whose {@code increment} keeps the count in memory, the same
 * shape as the Rust and TypeScript bench servers.
 *
 * <p>Env: RIVET_ENDPOINT (default http://127.0.0.1:6420), RIVET_POOL (required).
 */
public final class BenchServer {
    private BenchServer() {}

    @Name("counter")
    public interface BenchCounter extends Actor<None, None> {
        long increment(long amount);
    }

    public static final class BenchCounterActor implements BenchCounter {
        private long count;

        public BenchCounterActor(ActorContext<None> ctx) {}

        @Override
        public long increment(long amount) {
            count += amount;
            return count;
        }
    }

    public static void main(String[] args) {
        String pool = System.getenv("RIVET_POOL");
        if (pool == null) throw new IllegalStateException("RIVET_POOL is required");
        Registry.Serving serving = Registry.builder()
                .actor(BenchCounter.class, BenchCounterActor::new)
                .build()
                .serve(Registry.ServeOptions.fromEnv()
                        .withEndpoint(System.getenv().getOrDefault("RIVET_ENDPOINT", "http://127.0.0.1:6420"))
                        .withToken("default")
                        .withNamespace("default")
                        .withPoolName(pool)
                        .withEngineSpawn(Registry.EngineSpawn.NEVER));
        Runtime.getRuntime().addShutdownHook(new Thread(serving::close));
        serving.ready().join();
        System.out.println("bench server ready (java)");
        serving.done().join();
    }
}
