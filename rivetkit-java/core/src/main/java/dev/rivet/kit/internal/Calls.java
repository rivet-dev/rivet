package dev.rivet.kit.internal;

import dev.rivet.kit.Request;
import java.util.concurrent.Callable;
import java.util.function.Supplier;

/**
 * Per-call scope: which actor instance the current thread runs for, its turn
 * ticket, and the connection or request the call came in on. Bound with a
 * {@link ScopedValue} around each handler.
 */
final class Calls {
    private Calls() {}

    static final ScopedValue<Call> CURRENT = ScopedValue.newInstance();

    /** Mutable only by the thread running the call. */
    static final class Call {
        final Instance instance;
        Turn.Ticket ticket;
        int exclusive;
        final ConnImpl conn;
        final Request request;

        Call(Instance instance, Turn.Ticket ticket, ConnImpl conn, Request request) {
            this.instance = instance;
            this.ticket = ticket;
            this.conn = conn;
            this.request = request;
        }

        boolean holdsTurn() {
            return ticket != null && instance.turn.owns(ticket);
        }
    }

    static Call current() {
        return CURRENT.isBound() ? CURRENT.get() : null;
    }

    /** Throws unless the current thread holds {@code instance}'s turn. */
    static Call requireTurn(Instance instance, String what) {
        Call call = current();
        if (call == null || call.instance != instance || !call.holdsTurn()) {
            throw new IllegalStateException(what + " was called outside this actor's turn. Actor code runs in"
                    + " turns: use it from an action, hook, or a task started with ctx.spawn/waitUntil, not from"
                    + " a thread you started yourself.");
        }
        return call;
    }

    /**
     * Runs {@code wait} without the turn when the current thread holds one (and is
     * not inside {@code exclusive}), then takes a new place in line.
     */
    static <T> T outbound(Supplier<T> wait) {
        Call call = current();
        if (call == null || call.exclusive > 0 || !call.holdsTurn()) return wait.get();
        Turn turn = call.instance.turn;
        turn.release(call.ticket);
        call.ticket = null;
        try {
            return wait.get();
        } finally {
            Turn.Ticket ticket = turn.reserve();
            turn.await(ticket);
            call.ticket = ticket;
        }
    }

    static <T> T outboundChecked(Callable<T> work) {
        return outbound(() -> {
            try {
                return work.call();
            } catch (RuntimeException e) {
                throw e;
            } catch (Exception e) {
                throw new java.util.concurrent.CompletionException(e);
            }
        });
    }
}
