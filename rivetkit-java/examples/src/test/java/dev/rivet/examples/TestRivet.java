package dev.rivet.examples;

import static org.junit.jupiter.api.Assertions.assertTrue;

import dev.rivet.kit.internal.Native;
import dev.rivet.kit.testing.RivetTest;
import java.time.Duration;
import java.util.concurrent.TimeUnit;
import org.junit.jupiter.api.extension.BeforeAllCallback;
import org.junit.jupiter.api.extension.ExtensionContext;

/**
 * One registry (the examples plus test actors) and engine, shared by every test class. Classes
 * register with {@code @ExtendWith(TestRivet.class)}, which also checks once, after every class
 * has finished, that no calls were left waiting on Rust.
 */
final class TestRivet implements BeforeAllCallback {
    private static final Duration WARM_UP = Duration.ofSeconds(60);

    private static RivetTest rivet;
    private static RuntimeException startFailure;
    /** Calls pending (serving's `done`) and resources live while idle, for the leak check. */
    private static int idlePending;
    private static int idleResources;

    static synchronized RivetTest get() {
        if (startFailure != null) throw startFailure;
        if (rivet == null) {
            try {
                rivet = start();
            } catch (RuntimeException e) {
                startFailure = e;
                throw e;
            }
        }
        return rivet;
    }

    private static RivetTest start() {
        RivetTest started = RivetTest.start(Examples.builder()
                .actor(Lab.class, LabActor::new, actor -> actor
                        .migrate(LabActor::migrate)
                        .actionTimeout(Duration.ofSeconds(30))
                        .hookTimeout(Duration.ofSeconds(10))
                        .requestTimeout(Duration.ofSeconds(30))
                        .sleepTimeout(Duration.ofSeconds(60))
                        // Small, so the flood test overloads it with a few hundred frames.
                        .maxWaitingEvents(100))
                .build());
        // The engine and its store are thrown away after the run, so skip the graceful stop
        // (serving drain plus the engine's own ~10s storage shutdown) and kill the engine.
        Runtime.getRuntime().addShutdownHook(new Thread(TestRivet::killEngine));
        // The first call to a new pool waits for the engine to route to it; pay that once.
        long begin = System.nanoTime();
        long deadline = begin + WARM_UP.toNanos();
        Counter warm = started.client().getOrCreate(Counter.class, RivetTest.uniqueKey());
        while (true) {
            try {
                warm.get();
                break;
            } catch (RuntimeException e) {
                if (System.nanoTime() > deadline) {
                    throw new IllegalStateException("pool did not accept calls within " + WARM_UP, e);
                }
                System.err.println("warm-up attempt failed: " + e.getMessage());
                sleep(Duration.ofMillis(100));
            }
        }
        idlePending = Native.pendingCount();
        idleResources = Native.resourceCount();
        System.err.println("warm-up call took " + (System.nanoTime() - begin) / 1_000_000 + " ms");
        return started;
    }

    private static void killEngine() {
        ProcessHandle.current().children()
                .filter(p -> p.info().command().map(c -> c.contains("rivet-engine")).orElse(false))
                .forEach(engine -> {
                    engine.destroyForcibly();
                    try {
                        engine.onExit().get(5, TimeUnit.SECONDS);
                    } catch (Exception ignored) {
                        // exiting either way
                    }
                });
    }

    @Override
    public void beforeAll(ExtensionContext context) {
        context.getRoot().getStore(ExtensionContext.Namespace.GLOBAL)
                .computeIfAbsent(LeakCheck.class, k -> new LeakCheck(), LeakCheck.class);
    }

    /** Closed with the root context, after every test class has finished. */
    private static final class LeakCheck implements AutoCloseable {
        @Override
        public void close() {
            if (rivet == null) return;
            // At or below the idle baseline: something counted while idle may have ended since.
            for (int i = 0; i < 200 && (Native.pendingCount() > idlePending || Native.resourceCount() > idleResources); i++) {
                sleep(Duration.ofMillis(50));
            }
            assertTrue(Native.pendingCount() <= idlePending,
                    () -> "calls left waiting on Rust: " + Native.pendingCount() + " (idle: " + idlePending + ")");
            assertTrue(Native.resourceCount() <= idleResources,
                    () -> "streams, connections or sockets left open: " + Native.resourceTypes());
        }
    }

    private static void sleep(Duration d) {
        try {
            Thread.sleep(d);
        } catch (InterruptedException e) {
            Thread.currentThread().interrupt();
            throw new IllegalStateException("interrupted during warm-up", e);
        }
    }
}
