package dev.rivet.kit;

/**
 * A connection asks to receive an event. Throw to refuse.
 *
 * @param <E> the API's event type
 */
public interface OnSubscribe<E> {
    void onSubscribe(ActorConn<?> conn, Class<? extends E> event) throws Exception;
}
