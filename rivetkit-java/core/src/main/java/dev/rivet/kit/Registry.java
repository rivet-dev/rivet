package dev.rivet.kit;

import dev.rivet.kit.internal.ActorType;
import dev.rivet.kit.internal.ApiModel;
import dev.rivet.kit.internal.Native;
import dev.rivet.kit.internal.Wire;
import java.time.Duration;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
import java.util.Set;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.TimeoutException;
import java.util.function.Consumer;
import java.util.function.Function;

/**
 * The actor types this process serves.
 *
 * <pre>{@code
 * Registry registry = Registry.builder()
 *     .actor(ChatRoom.class, ChatRoomActor::new, actor -> actor.migrate(db -> db.script(SCHEMA)))
 *     .actor(Counter.class, CounterActor::new)
 *     .build();                                  // checks every API; fails fast
 * try (Registry.Serving serving = registry.serve()) {
 *     serving.ready().join();
 *     ...
 * }
 * }</pre>
 */
public final class Registry {
    private final List<Entry> entries;

    private record Entry(ActorType type, ActorOptions options) {}

    private Registry(List<Entry> entries) {
        this.entries = entries;
    }

    public static Builder builder() {
        return new Builder();
    }

    /** The actor names this registry serves. */
    public List<String> names() {
        return entries.stream().map(entry -> entry.type().model().name).toList();
    }

    /**
     * Checks every actor API (actions, events, params, wire types). {@link
     * Builder#build} already does; call this from a unit test to turn registration
     * errors into test failures without starting anything.
     */
    public Registry validate() {
        for (Entry entry : entries) ApiModel.of(entry.type().model().api);
        return this;
    }

    public Serving serve() {
        return serve(ServeOptions.fromEnv());
    }

    public Serving serve(ServeOptions options) {
        Native.init();
        long registry = Native.handleOf(Native.callSync(0, Wire.OP_REGISTRY_NEW, null, null));
        for (Entry entry : entries) {
            ActorType type = entry.type();
            ActorOptions o = entry.options();
            Native.registerType(type.id(), type);
            Native.callSync(registry, Wire.OP_REGISTER, new Wire.RegisterOptions(
                    type.model().name, type.id(), type.hooks(), millis(o.actionTimeout), millis(o.sleepTimeout),
                    o.noSleep ? Boolean.TRUE : null, millis(o.hookTimeout), millis(o.requestTimeout)), null);
        }
        CompletableFuture<Void> ready = Native.userFuture(Native.call(registry, Wire.OP_SERVE,
                new Wire.ServeOptions(options.endpoint(), options.token(), options.namespace(), options.poolName(),
                        options.engineSpawn() == null ? null : options.engineSpawn().name().toLowerCase()), null)
                .thenApply(answer -> (Void) null));
        CompletableFuture<Void> done = Native.userFuture(
                Native.call(registry, Wire.OP_SERVE_DONE, null, null).thenApply(answer -> (Void) null));
        return new Serving(registry, ready, done);
    }

    private static Integer millis(Duration duration) {
        return duration == null ? null : Math.toIntExact(duration.toMillis());
    }

    public static final class Builder {
        private final List<Entry> entries = new ArrayList<>();

        private Builder() {}

        /**
         * Registers an actor type. {@code factory} runs on every start (first start
         * and every wake) and receives the actor's context; javac checks that what
         * it builds implements {@code api}.
         */
        public <E, A extends Actor<E, ?>> Builder actor(Class<A> api, Function<ActorContext<A, E>, ? extends A> factory) {
            return actor(api, factory, options -> {});
        }

        @SuppressWarnings({"unchecked", "rawtypes"})
        public <E, A extends Actor<E, ?>> Builder actor(
                Class<A> api, Function<ActorContext<A, E>, ? extends A> factory, Consumer<ActorOptions> configure) {
            ActorOptions options = new ActorOptions();
            configure.accept(options);
            ApiModel model = ApiModel.of(api);
            entries.add(new Entry(new ActorType(model, (Function) factory, options.migrate), options));
            return this;
        }

        public Registry build() {
            Set<String> names = new HashSet<>();
            for (Entry entry : entries) {
                String name = entry.type().model().name;
                if (!names.add(name)) {
                    throw new IllegalArgumentException("two actor types are named `" + name + "`; use @Name to rename one");
                }
            }
            if (entries.isEmpty()) throw new IllegalArgumentException("register at least one actor");
            return new Registry(List.copyOf(entries));
        }
    }

    /** Per actor type settings. */
    public static final class ActorOptions {
        private Consumer<Db> migrate;
        private Duration actionTimeout;
        private Duration sleepTimeout;
        private Duration hookTimeout;
        private Duration requestTimeout;
        private boolean noSleep;

        private ActorOptions() {}

        /**
         * Sets up the schema. Runs on every start, before the constructor, so the
         * constructor can read the database. Make it idempotent
         * ({@code CREATE TABLE IF NOT EXISTS ...}).
         */
        public ActorOptions migrate(Consumer<Db> migrate) {
            this.migrate = migrate;
            return this;
        }

        /** How long an action may run (default 60s). */
        public ActorOptions actionTimeout(Duration timeout) {
            this.actionTimeout = timeout;
            return this;
        }

        /** Idle time before the actor sleeps (default 30s). */
        public ActorOptions sleepTimeout(Duration timeout) {
            this.sleepTimeout = timeout;
            return this;
        }

        /** How long each lifecycle hook may run (default 5s, 30s for migrate and wake). */
        public ActorOptions hookTimeout(Duration timeout) {
            this.hookTimeout = timeout;
            return this;
        }

        /** How long {@link OnRequest} may take to return the response head (default: the action timeout). */
        public ActorOptions requestTimeout(Duration timeout) {
            this.requestTimeout = timeout;
            return this;
        }

        /** Never sleep. */
        public ActorOptions noSleep() {
            this.noSleep = true;
            return this;
        }
    }

    public enum EngineSpawn {
        /** Spawn a local engine if none answers at the endpoint. */
        AUTO,
        ALWAYS,
        NEVER
    }

    /** Unset fields come from RIVET_ENDPOINT, RIVET_TOKEN, RIVET_NAMESPACE, RIVET_POOL. */
    public record ServeOptions(String endpoint, String token, String namespace, String poolName, EngineSpawn engineSpawn) {
        public static ServeOptions fromEnv() {
            return new ServeOptions(null, null, null, null, null);
        }

        public ServeOptions withEndpoint(String endpoint) {
            return new ServeOptions(endpoint, token, namespace, poolName, engineSpawn);
        }

        public ServeOptions withToken(String token) {
            return new ServeOptions(endpoint, token, namespace, poolName, engineSpawn);
        }

        public ServeOptions withNamespace(String namespace) {
            return new ServeOptions(endpoint, token, namespace, poolName, engineSpawn);
        }

        public ServeOptions withPoolName(String poolName) {
            return new ServeOptions(endpoint, token, namespace, poolName, engineSpawn);
        }

        public ServeOptions withEngineSpawn(EngineSpawn engineSpawn) {
            return new ServeOptions(endpoint, token, namespace, poolName, engineSpawn);
        }
    }

    /** A running registry. {@link #close} stops it: actors sleep, then serving ends. */
    public static final class Serving implements AutoCloseable {
        private final long registry;
        private final CompletableFuture<Void> ready;
        private final CompletableFuture<Void> done;
        private volatile boolean closed;

        private Serving(long registry, CompletableFuture<Void> ready, CompletableFuture<Void> done) {
            this.registry = registry;
            this.ready = ready;
            this.done = done;
        }

        /** Completes once the engine accepted this process, or fails with the reason. */
        public CompletableFuture<Void> ready() {
            return ready;
        }

        /** Completes when serving stops. */
        public CompletableFuture<Void> done() {
            return done;
        }

        /** Stops serving and waits up to 60s for actors to finish sleeping. Idempotent. */
        @Override
        public void close() {
            if (closed) return;
            closed = true;
            Native.callSync(registry, Wire.OP_STOP, null, null);
            try {
                done.get(60, TimeUnit.SECONDS);
            } catch (InterruptedException e) {
                Thread.currentThread().interrupt();
            } catch (TimeoutException e) {
                System.getLogger("dev.rivet.kit").log(System.Logger.Level.WARNING,
                        "rivetkit: actors did not stop within 60s");
            } catch (Exception ignored) {
                // done failed: serving is over either way
            }
            Native.release(registry);
        }
    }
}
