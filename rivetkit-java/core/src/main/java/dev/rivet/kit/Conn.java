package dev.rivet.kit;

import java.util.function.Consumer;

/**
 * A live connection to an actor: actions over one WebSocket, the actor's events,
 * and automatic reconnects. Listeners run on one thread per connection, in event
 * order (inside an actor: as turns of that actor). If listeners fall more than
 * {@value #MAX_QUEUED_EVENTS} events behind, the connection is closed with an error
 * rather than buffering without bound.
 *
 * @param <A> the actor API
 * @param <E> the actor's event type
 */
public interface Conn<A, E> extends AutoCloseable {
    int MAX_QUEUED_EVENTS = 1024;

    /** Actions over this connection. */
    A api();

    /** Listens for one event type. */
    <T extends E> Subscription on(Class<T> event, Consumer<? super T> listener);

    /** Listens for every event; switch over the sealed type to handle each. */
    Subscription onEvent(Consumer<? super E> listener);

    void onStatus(Consumer<Status> listener);

    /**
     * Connection-level errors, such as {@link OnBeforeConnect} rejecting this
     * connection. The connection keeps retrying (as the other clients do); close it
     * to stop. Actions wait while it is not connected.
     */
    void onError(Consumer<RivetException> listener);

    Status status();

    @Override
    void close();

    enum Status {
        IDLE,
        CONNECTING,
        CONNECTED,
        DISCONNECTED,
        /** Closed for good (by {@link #close} or because listeners fell behind). */
        CLOSED
    }

    interface Subscription extends AutoCloseable {
        @Override
        void close();
    }
}
