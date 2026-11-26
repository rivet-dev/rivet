package dev.rivet.kit;

import java.util.Map;
import java.util.concurrent.CompletableFuture;

/**
 * One actor, from the client side. Immutable: the {@code with*} methods return a
 * new handle.
 *
 * @param <A> the actor API
 */
public interface Handle<A> {
    /** The typed API. Calls block until the actor answers. */
    A api();

    /** Connection params sent with every call (checked against the API's params type). */
    Handle<A> withParams(Object params);

    /** Input for {@link OnCreate} if this call creates the actor. */
    Handle<A> withInput(Object input);

    /** Fail instead of creating the actor. */
    Handle<A> noCreate();

    /** Address the actor by id instead of name and key. */
    Handle<A> byId(String actorId);

    // Async calls by method reference: `handle.async(Counter::increment, 5L)`. The
    // future never completes on an SDK thread; continuations run on virtual threads.

    <R> CompletableFuture<R> async(Fn.Get0<A, R> action);

    <T1, R> CompletableFuture<R> async(Fn.Get1<A, T1, R> action, T1 a1);

    <T1, T2, R> CompletableFuture<R> async(Fn.Get2<A, T1, T2, R> action, T1 a1, T2 a2);

    <T1, T2, T3, R> CompletableFuture<R> async(Fn.Get3<A, T1, T2, T3, R> action, T1 a1, T2 a2, T3 a3);

    // Async calls to void actions: `handle.run(Counter::reset, 0L)`.

    CompletableFuture<Void> run(Fn.Call0<A> action);

    <T1> CompletableFuture<Void> run(Fn.Call1<A, T1> action, T1 a1);

    <T1, T2> CompletableFuture<Void> run(Fn.Call2<A, T1, T2> action, T1 a1, T2 a2);

    /** A raw HTTP request to the actor's {@link OnRequest}. The body streams as it arrives. */
    Response fetch(String method, String path, Map<String, String> headers, byte[] body);

    /** A raw WebSocket to the actor's {@link OnWebSocket}. */
    WebSocket webSocket(String path);
}
