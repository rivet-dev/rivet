package dev.rivet.kit.testing;

import dev.rivet.kit.Client;
import dev.rivet.kit.Registry;
import dev.rivet.kit.RivetException;
import java.time.Duration;
import java.util.UUID;
import java.util.concurrent.TimeUnit;
import java.util.function.Supplier;

/**
 * Serves a registry against a local engine (spawned, or reused when one already runs) on a fresh
 * pool, with a client pointed at it. Mirrors {@code rivetkit/src/test.rs}.
 *
 * <pre>{@code
 * try (RivetTest rivet = RivetTest.start(registry)) {
 *     Counter counter = rivet.client().getOrCreate(Counter.class, rivet.uniqueKey());
 *     assertEquals(5, rivet.retry(() -> counter.increment(5)));
 * }
 * }</pre>
 */
public final class RivetTest implements AutoCloseable {
    public static final String ENDPOINT = "http://127.0.0.1:6420";

    private final Registry.Serving serving;
    private final Client client;
    private final String pool;

    private RivetTest(Registry.Serving serving, Client client, String pool) {
        this.serving = serving;
        this.client = client;
        this.pool = pool;
    }

    public static RivetTest start(Registry registry) {
        String pool = "rivetkit-java-test-" + UUID.randomUUID();
        Registry.Serving serving = registry.serve(Registry.ServeOptions.fromEnv()
                .withEndpoint(ENDPOINT)
                .withToken("default")
                .withNamespace("default")
                .withPoolName(pool)
                .withEngineSpawn(Registry.EngineSpawn.ALWAYS));
        try {
            serving.ready().get(90, TimeUnit.SECONDS);
        } catch (Exception e) {
            serving.close();
            throw new IllegalStateException("registry did not become ready", e);
        }
        Client client = Client.of(new Client.Options(ENDPOINT, "default", "default", pool));
        return new RivetTest(serving, client, pool);
    }

    public Client client() {
        return client;
    }

    public String pool() {
        return pool;
    }

    public Registry.Serving serving() {
        return serving;
    }

    public static String uniqueKey() {
        return "k-" + UUID.randomUUID();
    }

    /**
     * Runs {@code call}, retrying while the engine reports the actor is still being
     * scheduled onto the new pool. Only the first call to a fresh actor needs it.
     */
    public <T> T retry(Supplier<T> call) {
        long deadline = System.nanoTime() + Duration.ofSeconds(30).toNanos();
        while (true) {
            try {
                return call.get();
            } catch (RivetException e) {
                String m = e.getMessage();
                boolean starting = m.contains("actor_ready_timeout") || m.contains("no_runner_config")
                        || m.contains("Service Unavailable") || m.contains("runner_config");
                if (!starting || System.nanoTime() > deadline) throw e;
                try {
                    Thread.sleep(250);
                } catch (InterruptedException ie) {
                    Thread.currentThread().interrupt();
                    throw e;
                }
            }
        }
    }

    @Override
    public void close() {
        client.close();
        serving.close();
    }
}
