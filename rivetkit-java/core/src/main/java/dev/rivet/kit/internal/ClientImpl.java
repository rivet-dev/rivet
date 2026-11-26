package dev.rivet.kit.internal;

import dev.rivet.kit.Actor;
import dev.rivet.kit.Client;
import dev.rivet.kit.Conn;
import dev.rivet.kit.Fn;
import dev.rivet.kit.Handle;
import dev.rivet.kit.Response;
import dev.rivet.kit.RivetException;
import dev.rivet.kit.WebSocket;
import java.lang.System.Logger.Level;
import java.lang.reflect.InvocationHandler;
import java.lang.reflect.Proxy;
import java.nio.charset.StandardCharsets;
import java.util.ArrayDeque;
import java.util.List;
import java.util.Map;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.CopyOnWriteArrayList;
import java.util.concurrent.atomic.AtomicReference;
import java.util.function.Consumer;
import java.util.function.Supplier;

/** {@link Client} over the wire. */
public final class ClientImpl implements Client {
    private static final System.Logger LOG = System.getLogger("dev.rivet.kit");

    final long handle;
    private volatile boolean closed;

    private ClientImpl(long handle) {
        this.handle = handle;
    }

    public static Client create(Options options) {
        Native.init();
        long handle = Native.handleOf(Native.callSync(0, Wire.OP_CLIENT_NEW,
                new Wire.ClientOptions(options.endpoint(), options.token(), options.namespace(), options.poolName()),
                null));
        return new ClientImpl(handle);
    }

    static Client wrap(long handle) {
        return new ClientImpl(handle);
    }

    @Override
    public <A extends Actor<?, ?>> A getOrCreate(Class<A> api, String... key) {
        return handle(api, key).api();
    }

    @Override
    public <A extends Actor<?, ?>> A get(Class<A> api, String... key) {
        return handle(api, key).noCreate().api();
    }

    @Override
    public <A extends Actor<?, ?>> Handle<A> handle(Class<A> api, String... key) {
        return new HandleImpl<>(this, ApiModel.of(api), List.of(key), null, false, null, null);
    }

    @Override
    public <E, A extends Actor<E, ?>> Conn<A, E> connect(Class<A> api, String... key) {
        return ConnClient.open(this, ApiModel.of(api), new HandleImpl<>(this, ApiModel.of(api), List.of(key), null,
                false, null, null).target());
    }

    @Override
    public <E, P, A extends Actor<E, P>> Conn<A, E> connect(
            Class<A> api, Supplier<? extends P> params, String... key) {
        HandleImpl<A> handle = new HandleImpl<>(this, ApiModel.of(api), List.of(key), null, false, null, null);
        return ConnClient.open(this, ApiModel.of(api), ((HandleImpl<A>) handle.withParams(params.get())).target());
    }

    @Override
    public void close() {
        if (closed) return;
        closed = true;
        Native.release(handle);
    }

    /** A proxy whose actions go through {@code call}. */
    static Object proxy(ApiModel model, ActionCall call, String label) {
        InvocationHandler handler = (proxy, method, args) -> {
            if (method.getDeclaringClass() == Object.class) {
                return switch (method.getName()) {
                    case "toString" -> label;
                    case "hashCode" -> System.identityHashCode(proxy);
                    case "equals" -> proxy == args[0];
                    default -> throw new UnsupportedOperationException(method.getName());
                };
            }
            if (method.isDefault()) return InvocationHandler.invokeDefault(proxy, method, args);
            ApiModel.Action action = model.action(method);
            byte[] body = action.args.encode(args);
            byte[] result = Calls.outbound(() -> call.call(action, body));
            return action.decodeResult(result);
        };
        return Proxy.newProxyInstance(model.api.getClassLoader(), new Class<?>[] {model.api}, handler);
    }

    @FunctionalInterface
    interface ActionCall {
        byte[] call(ApiModel.Action action, byte[] args);
    }

    // --- handles -------------------------------------------------------------------

    static final class HandleImpl<A> implements Handle<A> {
        private final ClientImpl client;
        private final ApiModel model;
        private final List<String> key;
        private final String actorId;
        private final boolean noCreate;
        private final byte[] params;
        private final byte[] input;
        private volatile A api;

        HandleImpl(ClientImpl client, ApiModel model, List<String> key, String actorId, boolean noCreate,
                byte[] params, byte[] input) {
            this.client = client;
            this.model = model;
            this.key = key;
            this.actorId = actorId;
            this.noCreate = noCreate;
            this.params = params;
            this.input = input;
        }

        Wire.ActorTarget target() {
            return new Wire.ActorTarget(model.name, key, actorId, noCreate, params, input);
        }

        @Override
        @SuppressWarnings("unchecked")
        public A api() {
            A current = api;
            if (current == null) {
                Wire.ActorTarget target = target();
                current = (A) proxy(model, (action, args) -> Native.callSync(client.handle, Wire.OP_CLIENT_ACTION,
                        new Wire.ClientActionCall(target, action.name), args).body(),
                        model.api.getSimpleName() + key);
                api = current;
            }
            return current;
        }

        @Override
        public Handle<A> withParams(Object params) {
            if (model.paramsType == null) {
                throw new IllegalArgumentException(model.api.getSimpleName() + " takes no connection params");
            }
            if (params != null && !model.paramsType.getRawClass().isInstance(params)) {
                throw new IllegalArgumentException(model.api.getSimpleName() + " takes params of type "
                        + model.paramsType.getRawClass().getName() + ", not " + params.getClass().getName());
            }
            return new HandleImpl<>(client, model, key, actorId, noCreate,
                    params == null ? null : Codec.encode(params), input);
        }

        @Override
        public Handle<A> withInput(Object input) {
            return new HandleImpl<>(client, model, key, actorId, noCreate, params,
                    input == null ? null : Codec.encode(input));
        }

        @Override
        public Handle<A> noCreate() {
            return new HandleImpl<>(client, model, key, actorId, true, params, input);
        }

        @Override
        public Handle<A> byId(String id) {
            return new HandleImpl<>(client, model, key, id, true, params, input);
        }

        private <R> CompletableFuture<R> invoke(Recorder.Invocation invocation) {
            Recorder.Recorded call = Recorder.record(model, invocation);
            CompletableFuture<Native.Answer> answer = Native.call(client.handle, Wire.OP_CLIENT_ACTION,
                    new Wire.ClientActionCall(target(), call.action().name), call.action().args.encode(call.args()));
            return answer.thenApplyAsync(a -> {
                @SuppressWarnings("unchecked")
                R result = (R) call.action().decodeResult(a.body());
                return result;
            }, Threads.VIRTUAL).exceptionallyCompose(error -> CompletableFuture.failedFuture(RivetException.from(error)));
        }

        @Override
        public <R> CompletableFuture<R> async(Fn.Get0<A, R> action) {
            return invoke(p -> action.call(ScheduleImpl.as(p)));
        }

        @Override
        public <T1, R> CompletableFuture<R> async(Fn.Get1<A, T1, R> action, T1 a1) {
            return invoke(p -> action.call(ScheduleImpl.as(p), a1));
        }

        @Override
        public <T1, T2, R> CompletableFuture<R> async(Fn.Get2<A, T1, T2, R> action, T1 a1, T2 a2) {
            return invoke(p -> action.call(ScheduleImpl.as(p), a1, a2));
        }

        @Override
        public <T1, T2, T3, R> CompletableFuture<R> async(Fn.Get3<A, T1, T2, T3, R> action, T1 a1, T2 a2, T3 a3) {
            return invoke(p -> action.call(ScheduleImpl.as(p), a1, a2, a3));
        }

        @Override
        public CompletableFuture<Void> run(Fn.Call0<A> action) {
            return invoke(p -> action.call(ScheduleImpl.as(p)));
        }

        @Override
        public <T1> CompletableFuture<Void> run(Fn.Call1<A, T1> action, T1 a1) {
            return invoke(p -> action.call(ScheduleImpl.as(p), a1));
        }

        @Override
        public <T1, T2> CompletableFuture<Void> run(Fn.Call2<A, T1, T2> action, T1 a1, T2 a2) {
            return invoke(p -> action.call(ScheduleImpl.as(p), a1, a2));
        }

        @Override
        public Response fetch(String method, String path, Map<String, String> headers, byte[] body) {
            AtomicReference<Streams.Inbound> stream = new AtomicReference<>();
            Native.Answer answer = Calls.outbound(() -> Native.await(Native.call(client.handle, Wire.OP_CLIENT_FETCH,
                    new Wire.ClientFetchCall(target(), method, path, headers == null ? Map.of() : headers), body,
                    a -> stream.set(new Streams.Inbound(Codec.decode(a.header(), Wire.FetchResult.class).body())))));
            Wire.FetchResult result = Codec.decode(answer.header(), Wire.FetchResult.class);
            return Response.received(result.status(), result.headers(), stream.get());
        }

        @Override
        public WebSocket webSocket(String path) {
            AtomicReference<ClientSocket> socket = new AtomicReference<>();
            Calls.outbound(() -> Native.await(Native.call(client.handle, Wire.OP_CLIENT_WEB_SOCKET,
                    new Wire.ClientWebSocketCall(target(), path, List.of()), null,
                    a -> socket.set(new ClientSocket(Native.handleOf(a), Dispatch.forCurrentCall())))));
            return socket.get();
        }
    }

    // --- event dispatch ---------------------------------------------------------------

    /**
     * Runs listener callbacks in order, one at a time: on one virtual thread, or as
     * turns of the actor that opened the connection. Bounded: a full queue reports
     * {@code false} and the caller closes the connection.
     */
    static final class Dispatch {
        private final Instance owner;
        private final ArrayDeque<Runnable> queue = new ArrayDeque<>();
        private boolean draining;
        private int inFlight;

        private Dispatch(Instance owner) {
            this.owner = owner;
        }

        static Dispatch forCurrentCall() {
            Calls.Call call = Calls.current();
            return new Dispatch(call == null ? null : call.instance);
        }

        synchronized boolean submit(Runnable task) {
            if (owner != null) {
                if (inFlight >= Conn.MAX_QUEUED_EVENTS) return false;
                inFlight++;
                owner.spawn(() -> {
                    try {
                        task.run();
                    } finally {
                        synchronized (this) {
                            inFlight--;
                        }
                    }
                }, 0);
                return true;
            }
            if (queue.size() >= Conn.MAX_QUEUED_EVENTS) return false;
            queue.addLast(task);
            if (!draining) {
                draining = true;
                Threads.start("rivet-events", this::drain);
            }
            return true;
        }

        private void drain() {
            while (true) {
                Runnable task;
                synchronized (this) {
                    task = queue.pollFirst();
                    if (task == null) {
                        draining = false;
                        return;
                    }
                }
                try {
                    task.run();
                } catch (Throwable t) {
                    LOG.log(Level.ERROR, "rivetkit: event listener failed", t);
                }
            }
        }
    }

    // --- connections -------------------------------------------------------------------

    static final class ConnClient<A, E> implements Conn<A, E>, Native.ResourceSink {
        private final ApiModel model;
        private final long id;
        private final Dispatch dispatch;
        private final Map<String, List<Consumer<Object>>> listeners = new ConcurrentHashMap<>();
        private final List<Consumer<Status>> statusListeners = new CopyOnWriteArrayList<>();
        private final List<Consumer<RivetException>> errorListeners = new CopyOnWriteArrayList<>();
        private volatile Status status = Status.CONNECTING;
        private volatile A api;

        private ConnClient(ApiModel model, long id, Dispatch dispatch) {
            this.model = model;
            this.id = id;
            this.dispatch = dispatch;
        }

        static <A, E> Conn<A, E> open(ClientImpl client, ApiModel model, Wire.ActorTarget target) {
            Dispatch dispatch = Dispatch.forCurrentCall();
            AtomicReference<ConnClient<A, E>> conn = new AtomicReference<>();
            Calls.outbound(() -> Native.await(Native.call(client.handle, Wire.OP_CLIENT_CONNECT,
                    new Wire.ClientConnectCall(target), null, answer -> {
                        ConnClient<A, E> created = new ConnClient<>(model, Native.handleOf(answer), dispatch);
                        Native.registerResource(created.id, created);
                        conn.set(created);
                    })));
            return conn.get();
        }

        @Override
        public void onEvent(int kind, byte[] header, byte[] body) {
            if (kind == Wire.CLIENT_EVENT) {
                String name = Codec.decode(header, Wire.ClientEventEvent.class).name();
                List<Consumer<Object>> targets = listeners.get(name);
                Codec.EventCodec codec = model.eventsByName.get(name);
                if (targets == null || codec == null) return;
                boolean accepted = dispatch.submit(() -> {
                    Object event = codec.decode(body);
                    for (Consumer<Object> listener : targets) listener.accept(event);
                });
                if (!accepted) {
                    LOG.log(Level.ERROR, "rivetkit: connection {0} closed: listeners fell more than {1} events behind",
                            id, Conn.MAX_QUEUED_EVENTS);
                    close();
                }
            } else if (kind == Wire.CLIENT_STATUS) {
                Wire.ClientStatusEvent event = Codec.decode(header, Wire.ClientStatusEvent.class);
                if (event.error() != null) {
                    RivetException error = new RivetException(
                            event.error().group(), event.error().code(), event.error().message());
                    for (Consumer<RivetException> listener : errorListeners) dispatch.submit(() -> listener.accept(error));
                    return;
                }
                Status next = switch (event.status()) {
                    case "idle" -> Status.IDLE;
                    case "connected" -> Status.CONNECTED;
                    case "disconnected" -> Status.DISCONNECTED;
                    default -> Status.CONNECTING;
                };
                if (status == Status.CLOSED) return;
                status = next;
                for (Consumer<Status> listener : statusListeners) dispatch.submit(() -> listener.accept(next));
            }
        }

        @Override
        @SuppressWarnings("unchecked")
        public A api() {
            A current = api;
            if (current == null) {
                current = (A) proxy(model, (action, args) -> Native.callSync(id, Wire.OP_CONN_ACTION,
                        new Wire.ConnActionCall(action.name), args).body(), model.api.getSimpleName() + "(conn)");
                api = current;
            }
            return current;
        }

        @Override
        @SuppressWarnings("unchecked")
        public <T extends E> Subscription on(Class<T> event, Consumer<? super T> listener) {
            Codec.EventCodec codec = model.event(event);
            return subscribe(List.of(codec.name), value -> ((Consumer<Object>) listener).accept(value));
        }

        @Override
        @SuppressWarnings("unchecked")
        public Subscription onEvent(Consumer<? super E> listener) {
            return subscribe(List.copyOf(model.eventsByName.keySet()),
                    value -> ((Consumer<Object>) listener).accept(value));
        }

        private Subscription subscribe(List<String> names, Consumer<Object> listener) {
            for (String name : names) {
                List<Consumer<Object>> targets = listeners.computeIfAbsent(name, n -> new CopyOnWriteArrayList<>());
                boolean first = targets.isEmpty();
                targets.add(listener);
                if (first) {
                    Calls.outbound(() -> Native.callSync(id, Wire.OP_CONN_SUBSCRIBE, new Wire.ConnSubscribeCall(name), null));
                }
            }
            return () -> {
                for (String name : names) {
                    List<Consumer<Object>> targets = listeners.get(name);
                    if (targets != null && targets.remove(listener) && targets.isEmpty() && status != Status.CLOSED) {
                        Native.call(id, Wire.OP_CONN_UNSUBSCRIBE, new Wire.ConnSubscribeCall(name), null);
                    }
                }
            };
        }

        @Override
        public void onStatus(Consumer<Status> listener) {
            statusListeners.add(listener);
        }

        @Override
        public void onError(Consumer<RivetException> listener) {
            errorListeners.add(listener);
        }

        @Override
        public Status status() {
            return status;
        }

        @Override
        public void close() {
            if (status == Status.CLOSED) return;
            status = Status.CLOSED;
            Native.unregisterResource(id);
            Native.release(id);
        }
    }

    // --- raw websockets ---------------------------------------------------------------

    static final class ClientSocket implements WebSocket, Native.ResourceSink {
        private final long id;
        private final Dispatch dispatch;
        private final List<Consumer<Message>> messageListeners = new CopyOnWriteArrayList<>();
        private final List<Consumer<Close>> closeListeners = new CopyOnWriteArrayList<>();
        private volatile boolean open = true;

        ClientSocket(long id, Dispatch dispatch) {
            this.id = id;
            this.dispatch = dispatch;
            Native.registerResource(id, this);
        }

        @Override
        public void onEvent(int kind, byte[] header, byte[] body) {
            if (kind == Wire.WS_MESSAGE) {
                Message message = new Message(Codec.decode(header, Wire.WsMessageEvent.class).binary(), body);
                if (!dispatch.submit(() -> messageListeners.forEach(l -> l.accept(message)))) {
                    LOG.log(Level.ERROR, "rivetkit: websocket {0} closed: listeners fell behind", id);
                    close(1008, "client too slow");
                }
            } else if (kind == Wire.WS_CLOSE) {
                open = false;
                Native.unregisterResource(id);
                Wire.WsCloseEvent event = Codec.decode(header, Wire.WsCloseEvent.class);
                Close close = new Close(event.code(), event.reason(), event.wasClean());
                dispatch.submit(() -> closeListeners.forEach(l -> l.accept(close)));
            }
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
    }
}
