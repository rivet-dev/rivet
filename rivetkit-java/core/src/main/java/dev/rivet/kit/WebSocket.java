package dev.rivet.kit;

import java.nio.charset.StandardCharsets;
import java.util.function.Consumer;

/**
 * A raw WebSocket, on the actor side ({@link OnWebSocket}) or the client side.
 * Actor-side listeners run as turns of the actor, in frame order; client-side
 * listeners run on one thread per socket, in frame order.
 */
public interface WebSocket extends AutoCloseable {
    void send(String text);

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
