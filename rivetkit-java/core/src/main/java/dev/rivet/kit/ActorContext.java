package dev.rivet.kit;

import java.time.Duration;
import java.util.List;
import java.util.Optional;
import java.util.concurrent.Callable;

/**
 * What an actor can do. Passed to the actor's constructor, which runs on every
 * start (first start and every wake); keep it in a field.
 *
 * <p><b>Turns.</b> At most one thread runs an actor's code at a time, so fields need
 * no locks. Each action, hook, message and spawned task holds the actor's turn while
 * it runs. {@link #db()} calls keep the turn, so a read-then-write in one action is
 * atomic. Waiting on something outside the actor gives the turn up until the wait
 * ends: calls through {@link #client()} and {@link #blocking}. Other work may
 * run meanwhile, so re-check fields after such a call, or wrap the section in
 * {@link #exclusive}. Calling any method here from a thread that does not hold the
 * turn throws {@link IllegalStateException}.
 *
 * @param <A> the actor's API (its own methods are what {@link #schedule} and {@link #cron} accept)
 * @param <E> the API's event type
 */
public interface ActorContext<A extends Actor<E, ?>, E> {
    String actorId();

    /** The actor type's name. */
    String name();

    List<String> key();

    String region();

    /** Sends {@code event} to every connection subscribed to it. */
    void broadcast(E event);

    /** Sends {@code event} to one connection. */
    void send(ActorConn<?> conn, E event);

    /** The connection the current action came in on, if any. */
    Optional<ActorConn<?>> conn();

    /** Open connections. */
    List<ActorConn<?>> conns();

    /** This actor's SQLite database. Calls keep the turn. */
    Db db();

    Schedule<A> schedule();

    Cron<A> cron();

    /** A client for calling other actors. Calls give up the turn while they wait. */
    Client client();

    /** Runs {@code work} on this thread, keeping the actor awake until it returns. */
    <T> T keepAwake(Callable<T> work);

    /**
     * Runs {@code work} in the background, as turns of this actor, and keeps the
     * actor from sleeping until it finishes (or the shutdown deadline passes).
     */
    void waitUntil(Runnable work);

    /**
     * Runs {@code work} in the background, as turns of this actor. It does not keep
     * the actor awake. It is interrupted when the actor starts shutting down.
     */
    void spawn(Runnable work);

    /** True once the actor started shutting down; background work should stop. */
    boolean aborted();

    /** Runs {@code work} without the turn, then takes the turn back. For slow I/O. */
    <T> T blocking(Callable<T> work);

    /** Runs {@code work} holding the turn even across outbound calls. Deadlock-prone; see the docs. */
    <T> T exclusive(Callable<T> work);

    /** Asks the actor to sleep once current work finishes. */
    void sleep();

    /** Destroys the actor once current work finishes. Its data is deleted. */
    void destroy();

    /** {@code Duration} helper so scheduling reads naturally. */
    static Duration seconds(long seconds) {
        return Duration.ofSeconds(seconds);
    }
}
