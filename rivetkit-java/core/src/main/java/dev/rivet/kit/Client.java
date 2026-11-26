package dev.rivet.kit;

import dev.rivet.kit.internal.ClientImpl;
import java.util.function.Supplier;

/**
 * Calls actors. Actor APIs come back as typed proxies whose methods block until the
 * actor answers (cheap on virtual threads); use {@link Handle#async} for futures.
 *
 * <pre>{@code
 * try (Client client = Client.of("http://127.0.0.1:6420")) {
 *     Counter counter = client.getOrCreate(Counter.class, "my-key");
 *     long n = counter.increment(5);
 * }
 * }</pre>
 *
 * <p>Inside an actor, use {@code ctx.client()}: its calls give up the actor's turn
 * while they wait, so actors can call each other (and back) without deadlocking.
 */
public interface Client extends AutoCloseable {
    static Client of(String endpoint) {
        return of(Options.of(endpoint));
    }

    static Client of(Options options) {
        return ClientImpl.create(options);
    }

    /** The actor for {@code key}, created if it does not exist. */
    <A extends Actor<?, ?>> A getOrCreate(Class<A> api, String... key);

    /** The actor for {@code key}; calls fail with {@code actor/not_found} if it does not exist. */
    <A extends Actor<?, ?>> A get(Class<A> api, String... key);

    /** A handle for options (input, by id), async calls, raw HTTP and WebSockets. */
    <A extends Actor<?, ?>> Handle<A> handle(Class<A> api, String... key);

    /** A handle whose calls carry {@code params}, which must be the API's params type. */
    <P, A extends Actor<?, P>> Handle<A> handleWithParams(Class<A> api, P params, String... key);

    /** A live connection: actions over one WebSocket, plus the actor's events. */
    <E, A extends Actor<E, ?>> Conn<A, E> connect(Class<A> api, String... key);

    /**
     * A live connection with params. {@code params} is called before every dial, the first
     * connect and each reconnect, so it can return a fresh token. It runs on a virtual thread
     * and may block; if it throws, the connection reports the error and retries with backoff.
     */
    <E, P, A extends Actor<E, P>> Conn<A, E> connect(Class<A> api, Supplier<? extends P> params, String... key);

    @Override
    void close();

    record Options(String endpoint, String token, String namespace, String poolName) {
        public static Options of(String endpoint) {
            return new Options(endpoint, null, null, null);
        }

        /** From RIVET_ENDPOINT (default http://127.0.0.1:6420), RIVET_TOKEN, RIVET_NAMESPACE, RIVET_POOL. */
        public static Options fromEnv() {
            String endpoint = System.getenv().getOrDefault("RIVET_ENDPOINT", "http://127.0.0.1:6420");
            return new Options(endpoint, System.getenv("RIVET_TOKEN"), System.getenv("RIVET_NAMESPACE"),
                    System.getenv("RIVET_POOL"));
        }

        public Options withToken(String token) {
            return new Options(endpoint, token, namespace, poolName);
        }

        public Options withNamespace(String namespace) {
            return new Options(endpoint, token, namespace, poolName);
        }

        public Options withPoolName(String poolName) {
            return new Options(endpoint, token, namespace, poolName);
        }
    }
}
