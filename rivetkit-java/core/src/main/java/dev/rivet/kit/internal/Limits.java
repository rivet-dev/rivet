package dev.rivet.kit.internal;

/** Where the SDK refuses to queue more, and how it says so. */
final class Limits {
    private Limits() {}

    /**
     * Close code for a WebSocket whose receiver cannot keep up: an actor with too many events
     * waiting, or client listeners too far behind. 1013 is "try again later".
     */
    static final int CLOSE_OVERLOADED = 1013;
}
