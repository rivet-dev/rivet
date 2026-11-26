package dev.rivet.examples;

import dev.rivet.kit.ActorConn;
import dev.rivet.kit.ActorContext;
import dev.rivet.kit.None;
import dev.rivet.kit.OnWebSocket;
import dev.rivet.kit.Request;
import dev.rivet.kit.WebSocket;
import java.util.Optional;

public final class EchoActor implements Echo, OnWebSocket {
    private int frames;

    public EchoActor(ActorContext<None> ctx) {}

    @Override
    public void onWebSocket(WebSocket socket, ActorConn<?> conn, Optional<Request> request) {
        socket.onMessage(message -> {
            frames++;
            if (message.binary()) socket.send(message.data());
            else socket.send(frames + ":" + message.text());
        });
    }

    @Override
    public int frames() {
        return frames;
    }
}
