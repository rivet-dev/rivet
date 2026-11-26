package dev.rivet.examples;

import dev.rivet.kit.Actor;
import dev.rivet.kit.None;

/** A raw WebSocket that echoes frames back, prefixed with their sequence number. */
public interface Echo extends Actor<None, None> {
    int frames();
}
