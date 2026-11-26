package dev.rivet.kit.internal;

import dev.rivet.kit.WebSocket;
import java.nio.charset.StandardCharsets;
import java.util.List;
import java.util.concurrent.CopyOnWriteArrayList;
import java.util.function.Consumer;

/** An actor-side raw WebSocket. Listeners run as turns of the actor, in frame order. */
final class ServerSocket implements WebSocket {
    private final Instance instance;
    private final long id;
    private final List<Consumer<Message>> messageListeners = new CopyOnWriteArrayList<>();
    private final List<Consumer<Close>> closeListeners = new CopyOnWriteArrayList<>();
    private volatile boolean open = true;

    ServerSocket(Instance instance, long id) {
        this.instance = instance;
        this.id = id;
    }

    @Override
    public void send(String text) {
        send(false, text.getBytes(StandardCharsets.UTF_8));
    }

    @Override
    public void send(byte[] data) {
        send(true, data);
    }

    private void send(boolean binary, byte[] data) {
        if (!open) throw new IllegalStateException("websocket is closed");
        Native.callSync(id, Wire.OP_WS_SEND, new Wire.WsSendCall(binary), data);
    }

    @Override
    public void close(int code, String reason) {
        if (!open) return;
        open = false;
        Native.call(id, Wire.OP_WS_CLOSE, new Wire.WsCloseCall(code, reason), null);
    }

    @Override
    public void onMessage(Consumer<Message> listener) {
        messageListeners.add(listener);
    }

    @Override
    public void onClose(Consumer<Close> listener) {
        closeListeners.add(listener);
    }

    void dispatchMessage(boolean binary, byte[] data) {
        Message message = new Message(binary, data);
        for (Consumer<Message> listener : messageListeners) listener.accept(message);
    }

    void dispatchClose(int code, String reason, boolean wasClean) {
        open = false;
        Close close = new Close(code, reason, wasClean);
        for (Consumer<Close> listener : closeListeners) listener.accept(close);
    }

    void closed() {
        open = false;
    }

    @Override
    public String toString() {
        return "WebSocket[" + id + " of " + instance.type.model.name + "]";
    }
}
