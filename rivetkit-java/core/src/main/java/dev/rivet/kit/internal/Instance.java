package dev.rivet.kit.internal;

import dev.rivet.kit.ActorConn;
import dev.rivet.kit.ActorContext;
import dev.rivet.kit.OnBeforeConnect;
import dev.rivet.kit.OnConnect;
import dev.rivet.kit.OnCreate;
import dev.rivet.kit.OnDestroy;
import dev.rivet.kit.OnDisconnect;
import dev.rivet.kit.OnFork;
import dev.rivet.kit.OnRequest;
import dev.rivet.kit.OnSleep;
import dev.rivet.kit.OnSubscribe;
import dev.rivet.kit.OnWake;
import dev.rivet.kit.OnWebSocket;
import dev.rivet.kit.Request;
import dev.rivet.kit.Response;
import dev.rivet.kit.RivetException;
import java.io.BufferedOutputStream;
import java.io.ByteArrayInputStream;
import java.io.InputStream;
import java.io.SequenceInputStream;
import java.lang.System.Logger.Level;
import java.util.HashMap;
import java.util.Locale;
import java.util.Map;
import java.util.Optional;
import java.util.Set;
import java.util.concurrent.ConcurrentHashMap;

/**
 * One running actor. Every event becomes a task that runs on its own virtual
 * thread holding the actor's turn, in arrival order. See {@link Turn}.
 */
final class Instance {
    private static final System.Logger LOG = System.getLogger("dev.rivet.kit");
    final ActorType type;
    final long handle;
    final Turn turn = new Turn();
    final DbImpl db;
    volatile Object impl;
    volatile Wire.ActorInfo info;
    volatile ContextImpl ctx;
    volatile boolean aborted;
    volatile boolean released;
    final Map<String, ConnImpl> conns = new ConcurrentHashMap<>();
    final Map<Long, ServerSocket> sockets = new ConcurrentHashMap<>();
    private final Map<Long, Thread> running = new ConcurrentHashMap<>();
    private final Set<Long> cancelled = ConcurrentHashMap.newKeySet();
    private final Set<Thread> background = ConcurrentHashMap.newKeySet();
    /** Request bodies registered on the upcall thread, before their first chunk. */
    private final Map<Long, InputStream> requestBodies = new ConcurrentHashMap<>();

    Instance(ActorType type, long handle) {
        this.type = type;
        this.handle = handle;
        this.db = new DbImpl(handle);
    }

    ApiModel model() {
        return type.model;
    }

    // --- arrival (Rust thread: enqueue only) -------------------------------------

    void submit(int kind, long event, byte[] header, byte[] body) {
        if (kind == Wire.REQUEST) prepareRequest(event, header, body);
        if (kind == Wire.WS_MESSAGE) {
            // Frames get no reply, so they cannot be refused one by one like other events:
            // past the limit the socket is closed instead, and later frames are dropped.
            ServerSocket socket = sockets.get(event);
            if (socket == null || !socket.isOpen()) return;
            if (turn.waiting() >= type.maxWaiting) {
                socket.overloaded();
                return;
            }
        }
        boolean concurrent = kind == Wire.ACTION && isConcurrent(header);
        boolean reply = kind != Wire.WS_MESSAGE;
        if (!concurrent && turn.waiting() >= type.maxWaiting && reply) {
            if (kind == Wire.REQUEST) closeQuietly(requestBodies.remove(event));
            Threads.VIRTUAL.execute(() -> Native.replyError(event, new RivetException(
                    "actor", "overloaded", "the actor has " + type.maxWaiting + " events waiting; try again later")));
            return;
        }
        Turn.Ticket ticket = concurrent ? null : turn.reserve();
        Threads.start("rivet-" + type.model.name, () -> run(kind, event, header, body, ticket));
    }

    private boolean isConcurrent(byte[] header) {
        Wire.ActionEvent action = Codec.decode(header, Wire.ActionEvent.class);
        ApiModel.Action known = type.model.actions.get(action.name());
        return known != null && known.concurrent;
    }

    private void prepareRequest(long event, byte[] header, byte[] initial) {
        Wire.RequestEvent request = Codec.decode(header, Wire.RequestEvent.class);
        InputStream body = new ByteArrayInputStream(initial);
        if (request.bodyStream() != null) {
            body = new SequenceInputStream(body, new Streams.Inbound(request.bodyStream()));
        }
        requestBodies.put(event, body);
    }

    void cancel(long event) {
        Thread thread = running.get(event);
        if (thread != null) thread.interrupt();
        else cancelled.add(event);
    }

    void abort() {
        aborted = true;
        for (Thread thread : background) thread.interrupt();
    }

    void released() {
        released = true;
        abort();
        for (ServerSocket socket : sockets.values()) socket.closed();
        sockets.clear();
        conns.clear();
        for (InputStream body : requestBodies.values()) {
            try {
                body.close();
            } catch (Exception ignored) {
                // gone anyway
            }
        }
        requestBodies.clear();
        ContextImpl context = ctx;
        if (context != null) context.released();
    }

    // --- running (virtual thread, holding the turn) -----------------------------

    private void run(int kind, long event, byte[] header, byte[] body, Turn.Ticket ticket) {
        if (ticket != null) turn.await(ticket);
        Calls.Call call = new Calls.Call(this, ticket, connOf(kind, header), null);
        boolean reply = kind != Wire.WS_MESSAGE;
        try {
            if (cancelled.remove(event)) {
                // Rust stopped waiting before this ran; a request's body would stay leased.
                if (kind == Wire.REQUEST) closeQuietly(requestBodies.remove(event));
                return;
            }
            if (reply) running.put(event, Thread.currentThread());
            ScopedValue.where(Calls.CURRENT, call).call(() -> {
                dispatch(kind, event, header, body, call);
                return null;
            });
        } catch (Throwable t) {
            if (reply) Native.replyError(event, t);
            else LOG.log(Level.ERROR, "rivetkit: websocket listener failed", t);
        } finally {
            running.remove(event);
            Thread.interrupted();
            if (call.ticket != null && turn.owns(call.ticket)) turn.release(call.ticket);
        }
    }

    private ConnImpl connOf(int kind, byte[] header) {
        if (kind != Wire.ACTION) return null;
        String id = Codec.decode(header, Wire.ActionEvent.class).connId();
        return id == null ? null : conns.get(id);
    }

    @SuppressWarnings({"unchecked", "rawtypes"})
    private void dispatch(int kind, long event, byte[] header, byte[] body, Calls.Call call) throws Throwable {
        Object actor = impl;
        if (kind == Wire.MIGRATE) {
            type.migrate.accept(db);
            reply(event);
        } else if (kind == Wire.START) {
            info = Codec.decode(header, Wire.StartEvent.class).info();
            ctx = new ContextImpl(this);
            Object created = type.factory.apply(ctx);
            if (created == null) throw new IllegalStateException("the actor factory returned null");
            if (!type.model.api.isInstance(created)) {
                throw new IllegalStateException(created.getClass().getName() + " does not implement " + type.model.api.getName());
            }
            ImplModel.of(created.getClass()).check(type.model);
            impl = created;
            reply(event);
        } else if (kind == Wire.CREATE) {
            if (actor instanceof OnCreate hook) {
                Wire.CreateEvent create = Codec.decode(header, Wire.CreateEvent.class);
                Object input = create.hasInput() ? ImplModel.of(actor.getClass()).decodeInput(body) : null;
                hook.onCreate(input);
            }
            reply(event);
        } else if (kind == Wire.FORK) {
            if (actor instanceof OnFork hook) {
                Wire.ForkEvent fork = Codec.decode(header, Wire.ForkEvent.class);
                hook.onFork(new OnFork.Fork(fork.parentActorId(), fork.parentKey(), fork.forkedAt(),
                        fork.hasInput() ? body : null));
            }
            reply(event);
        } else if (kind == Wire.WAKE) {
            if (actor instanceof OnWake hook) hook.onWake();
            reply(event);
        } else if (kind == Wire.ACTION) {
            Wire.ActionEvent request = Codec.decode(header, Wire.ActionEvent.class);
            ApiModel.Action action = type.model.actions.get(request.name());
            if (action == null) {
                throw new RivetException("actor", "action_not_found", "no action named `" + request.name() + "`");
            }
            Object[] args = action.args.decode(body);
            Object result = action.invoker.invoke(actor, args);
            Native.reply(event, null, action.encodeResult(result));
        } else if (kind == Wire.BEFORE_CONNECT) {
            Wire.ConnEvent connect = Codec.decode(header, Wire.ConnEvent.class);
            Object params = type.model.paramsType == null || body.length == 0
                    ? null
                    : Codec.decode(body, type.model.paramsType);
            ConnImpl conn = new ConnImpl(this, connect.connId(), params);
            conns.put(conn.id(), conn);
            try {
                if (actor instanceof OnBeforeConnect hook) hook.onBeforeConnect(params, Optional.ofNullable(request(connect.request(), null)));
            } catch (Throwable t) {
                conns.remove(conn.id());
                throw t;
            }
            reply(event);
        } else if (kind == Wire.CONNECT) {
            Wire.ConnEvent connect = Codec.decode(header, Wire.ConnEvent.class);
            ConnImpl conn = conns.computeIfAbsent(connect.connId(), id -> new ConnImpl(this, id, null));
            if (actor instanceof OnConnect hook) hook.onConnect(conn);
            reply(event);
        } else if (kind == Wire.DISCONNECT) {
            Wire.ConnEvent disconnect = Codec.decode(header, Wire.ConnEvent.class);
            ConnImpl conn = conns.remove(disconnect.connId());
            if (conn != null && actor instanceof OnDisconnect hook) hook.onDisconnect(conn);
            reply(event);
        } else if (kind == Wire.SUBSCRIBE) {
            Wire.SubscribeEvent subscribe = Codec.decode(header, Wire.SubscribeEvent.class);
            Codec.EventCodec codec = type.model.eventsByName.get(subscribe.event());
            if (codec == null && !type.model.eventsByName.isEmpty()) {
                throw new RivetException("actor", "event_not_found", "no event named `" + subscribe.event() + "`");
            }
            if (codec != null && actor instanceof OnSubscribe hook) {
                ConnImpl conn = conns.computeIfAbsent(subscribe.connId(), id -> new ConnImpl(this, id, null));
                hook.onSubscribe(conn, (Class) codec.type);
            }
            reply(event);
        } else if (kind == Wire.REQUEST) {
            handleRequest(actor, event, header);
        } else if (kind == Wire.WEB_SOCKET) {
            Wire.WebSocketEvent open = Codec.decode(header, Wire.WebSocketEvent.class);
            if (!(actor instanceof OnWebSocket hook)) {
                throw new RivetException("actor", "not_configured", "this actor does not handle WebSockets");
            }
            ServerSocket socket = new ServerSocket(this, open.ws());
            sockets.put(open.ws(), socket);
            ConnImpl conn = conns.computeIfAbsent(open.connId(), id -> new ConnImpl(this, id, null));
            hook.onWebSocket(socket, conn, Optional.ofNullable(request(open.request(), null)));
            reply(event);
        } else if (kind == Wire.WS_MESSAGE) {
            ServerSocket socket = sockets.get(event);
            if (socket != null) {
                Wire.WsMessageEvent message = Codec.decode(header, Wire.WsMessageEvent.class);
                socket.dispatchMessage(message.binary(), body);
            }
        } else if (kind == Wire.WS_CLOSE) {
            Wire.WsCloseEvent close = Codec.decode(header, Wire.WsCloseEvent.class);
            ServerSocket socket = sockets.remove(close.ws());
            if (socket != null) socket.dispatchClose(close.code(), close.reason(), close.wasClean());
            reply(event);
        } else if (kind == Wire.SLEEP) {
            if (actor instanceof OnSleep hook) hook.onSleep();
            reply(event);
        } else if (kind == Wire.DESTROY) {
            if (actor instanceof OnDestroy hook) hook.onDestroy();
            reply(event);
        } else {
            throw new RivetException("actor", "unknown_event", "unknown event kind " + kind);
        }
    }

    private static void reply(long event) {
        Native.reply(event, null, null);
    }

    private Request request(Wire.RequestHead head, InputStream body) {
        if (head == null) return null;
        Map<String, String> headers = new HashMap<>();
        if (head.headers() != null) {
            head.headers().forEach((name, value) -> headers.put(name.toLowerCase(Locale.ROOT), value));
        }
        return new Request(head.method(), head.uri(), Map.copyOf(headers),
                body == null ? InputStream.nullInputStream() : body);
    }

    private void handleRequest(Object actor, long event, byte[] header) throws Exception {
        Wire.RequestEvent requestEvent = Codec.decode(header, Wire.RequestEvent.class);
        InputStream body = requestBodies.remove(event);
        Request request = request(requestEvent.request(), body);
        // The request body is closed once the response is done, read or not: an unread
        // streamed body would otherwise keep its chunks (up to the 256 KiB window) leased
        // and its Rust stream open. A streaming response hands it to the writer task.
        boolean handedOff = false;
        try {
            if (!(actor instanceof OnRequest hook)) {
                throw new RivetException("actor", "not_configured", "this actor does not handle HTTP requests");
            }
            Response response = hook.onRequest(request);
            if (response == null) throw new IllegalStateException("onRequest returned null");
            switch (response) {
                case Response.Bytes whole -> Native.reply(event,
                        Codec.encode(new Wire.RequestReply(whole.status(), whole.headers(), false)), whole.body());
                case Response.Streaming streaming -> {
                    // Before the head: if the actor is stopping this throws, and the client
                    // gets an error instead of a head whose body never ends.
                    long region = ctx.keepAwakeRegion();
                    Native.reply(event,
                            Codec.encode(new Wire.RequestReply(streaming.status(), streaming.headers(), true)), null);
                    stream(streaming.writer(), new Streams.Outbound(requestEvent.responseBody()), request.body(), region);
                    handedOff = true;
                }
            }
        } finally {
            if (!handedOff) closeQuietly(request.body());
        }
    }

    /** Runs a streaming response's writer as a background turn; the writer may still read the request body. */
    private void stream(Response.Writer writer, Streams.Outbound out, InputStream requestBody, long region) {
        spawn(() -> {
            BufferedOutputStream buffered = new BufferedOutputStream(out, 64 * 1024);
            try {
                writer.write(buffered);
            } catch (Exception e) {
                // Abort without closing: closing would end the body cleanly.
                LOG.log(Level.WARNING, "rivetkit: streaming response failed", e);
                out.abort(String.valueOf(e.getMessage()));
                return;
            } finally {
                closeQuietly(requestBody);
            }
            try {
                buffered.close();
            } catch (Exception e) {
                LOG.log(Level.DEBUG, "rivetkit: client went away before the response ended", e);
            }
        }, region);
    }

    private static void closeQuietly(InputStream in) {
        if (in == null) return;
        try {
            in.close();
        } catch (Exception ignored) {
            // releasing either way
        }
    }

    /** Runs {@code work} as a background turn; releases {@code region} when it ends. */
    void spawn(Runnable work, long region) {
        Turn.Ticket ticket = turn.reserve();
        Threads.start("rivet-" + type.model.name + "-task", () -> {
            Thread self = Thread.currentThread();
            background.add(self);
            if (aborted) self.interrupt();
            turn.await(ticket);
            Calls.Call call = new Calls.Call(this, ticket, null, null);
            try {
                ScopedValue.where(Calls.CURRENT, call).run(work);
            } catch (Throwable t) {
                if (!aborted) LOG.log(Level.ERROR, "rivetkit: background task failed", t);
            } finally {
                background.remove(self);
                if (call.ticket != null && turn.owns(call.ticket)) turn.release(call.ticket);
                if (region != 0) Native.release(region);
            }
        });
    }

    ActorConn<?> currentConn() {
        Calls.Call call = Calls.current();
        return call == null || call.instance != this ? null : call.conn;
    }
}
