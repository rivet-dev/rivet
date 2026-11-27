package dev.rivet.kit;

/**
 * A client connection to this actor, as the actor sees it.
 *
 * @param <P> the connection params type
 */
public interface ActorConn<P> {
    String id();

    /** The params the client connected with, or null when it sent none. */
    P params();

    /** Closes the connection; the client sees {@code reason}. */
    void disconnect(String reason);
}
