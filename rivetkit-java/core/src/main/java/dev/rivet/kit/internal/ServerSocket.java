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
    private boolean closeDispatched;

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

    /** Core reports only closes the client starts, so a close from this side runs the listeners here. */
    @Override
    public void close(int code, String reason) {
        if (!open) return;
        open = false;
        Native.call(id, Wire.OP_WS_CLOSE, new Wire.WsCloseCall(code, reason), null);
        dispatchClose(code, reason, true);
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
        synchronized (this) {
            if (closeDispatched) return;
            closeDispatched = true;
        }
        Close close = new Close(code, reason, wasClean);
        for (Consumer<Close> listener : closeListeners) listener.accept(close);
    }

    void closed() {
        open = false;
    }

    boolean isOpen() {
        return open;
    }

    /**
     * The actor has too many events waiting: close with 1013 (try again later). The close
     * frame goes out at once; the close listeners run as a turn, like every listener.
     */
    synchronized void overloaded() {
        if (!open) return;
        open = false;
        String reason = "actor.overloaded";
        Threads.VIRTUAL.execute(() ->
                Native.call(id, Wire.OP_WS_CLOSE, new Wire.WsCloseCall(Limits.CLOSE_OVERLOADED, reason), null));
        instance.spawn(() -> dispatchClose(Limits.CLOSE_OVERLOADED, reason, false), 0);
    }

    @Override
    public String toString() {
        return "WebSocket[" + id + " of " + instance.type.model.name + "]";
    }
}
