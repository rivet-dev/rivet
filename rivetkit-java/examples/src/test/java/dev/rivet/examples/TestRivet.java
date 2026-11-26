package dev.rivet.examples;

import dev.rivet.kit.testing.RivetTest;
import java.time.Duration;
import java.util.function.BooleanSupplier;

/** One registry (the examples plus test actors) and engine, shared by every test class. */
final class TestRivet {
    private static RivetTest rivet;
    /** Calls pending while idle (serving's `done`), so leak checks compare against it. */
    static int idlePending;

    private TestRivet() {}

    static synchronized RivetTest get() {
        if (rivet == null) {
            rivet = RivetTest.start(Examples.builder()
                    .actor(Lab.class, LabActor::new, actor -> actor
                            .migrate(LabActor::migrate)
                            .actionTimeout(Duration.ofSeconds(30))
                            .hookTimeout(Duration.ofSeconds(10))
                            .requestTimeout(Duration.ofSeconds(30))
                            .sleepTimeout(Duration.ofSeconds(60)))
                    .build());
            RivetTest started = rivet;
            Runtime.getRuntime().addShutdownHook(new Thread(started::close));
            // The first call to a new pool waits for the engine to route to it; pay that once.
            long begin = System.nanoTime();
            Counter warm = started.client().getOrCreate(Counter.class, RivetTest.uniqueKey());
            while (true) {
                long attempt = System.nanoTime();
                try {
                    warm.get();
                    break;
                } catch (RuntimeException e) {
                    System.err.println("warm-up attempt failed after " + (System.nanoTime() - attempt) / 1_000_000
                            + " ms: " + e.getMessage());
                }
            }
            idlePending = dev.rivet.kit.internal.Native.pendingCount();
            System.err.println("warm-up call took " + (System.nanoTime() - begin) / 1_000_000 + " ms");
        }
        return rivet;
    }

    static void eventually(BooleanSupplier condition, Duration timeout) throws InterruptedException {
        long deadline = System.nanoTime() + timeout.toNanos();
        while (!condition.getAsBoolean()) {
            if (System.nanoTime() > deadline) throw new AssertionError("condition not met within " + timeout);
            Thread.sleep(100);
        }
    }
}
