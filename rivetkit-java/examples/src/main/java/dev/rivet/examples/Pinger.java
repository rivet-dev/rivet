package dev.rivet.examples;

import dev.rivet.kit.Actor;
import dev.rivet.kit.None;

/** Exercises turns, schedules and actor-to-actor calls. */
public interface Pinger extends Actor<None, None> {
    /** Bounces between this actor and {@code peer} {@code hops} times. Needs re-entrancy. */
    int bounce(String peer, String self, int hops);

    /** A read-modify-write on a field with a deliberate pause; turns keep it atomic. */
    int bump();

    int bumps();

    void tickLater(long delayMillis);

    void tick();

    long ticks();

    /** A @Concurrent read. */
    @dev.rivet.kit.Concurrent
    long ticksConcurrently();
}
