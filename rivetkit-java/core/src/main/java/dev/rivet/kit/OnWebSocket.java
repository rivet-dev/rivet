package dev.rivet.kit;

import java.util.Optional;

/**
 * Handles raw WebSockets to the actor. Register listeners on {@code socket} before
 * returning; they run as turns of the actor, in frame order.
 */
public interface OnWebSocket {
    void onWebSocket(WebSocket socket, ActorConn<?> conn, Optional<Request> request) throws Exception;
}
