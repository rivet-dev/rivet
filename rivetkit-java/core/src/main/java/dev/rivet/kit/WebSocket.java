package dev.rivet.kit;

import java.nio.charset.StandardCharsets;
import java.util.function.Consumer;

/**
 * A raw WebSocket, on the actor side ({@link OnWebSocket}) or the client side.
 * Actor-side listeners run as turns of the actor, in frame order; client-side
 * listeners run on one thread per socket, in frame order.
 */
public interface WebSocket extends AutoCloseable {
    /**
     * Sends a frame. On the client side this waits while the peer is slow (a small queue is
     * full), so a fast sender cannot grow memory without bound; inside an actor it gives up the
     * turn while it waits. Actor-side sends do not wait: Rivet's server sockets have no send
     * backpressure.
     */
    void send(String text);

    /** Sends a binary frame; see {@link #send(String)}. */
    void send(byte[] data);

    void close(int code, String reason);

    @Override
    default void close() {
        close(1000, "");
    }

    void onMessage(Consumer<Message> listener);

    void onClose(Consumer<Close> listener);

    record Message(boolean binary, byte[] data) {
        public String text() {
            return new String(data, StandardCharsets.UTF_8);
        }
    }

    record Close(int code, String reason, boolean wasClean) {}
}
