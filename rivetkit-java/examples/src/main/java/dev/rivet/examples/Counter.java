package dev.rivet.examples;

import dev.rivet.kit.Actor;
import dev.rivet.kit.None;

/** A durable counter. The count lives in SQLite; every change is broadcast. */
public interface Counter extends Actor<Counter.NewCount, None> {
    long increment(long amount);

    long get();

    /** Emitted after every change. */
    record NewCount(long count) {}
}
