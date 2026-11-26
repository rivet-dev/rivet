package dev.rivet.kit.internal;

import java.util.ArrayDeque;
import java.util.concurrent.locks.LockSupport;

/**
 * An actor's turn: at most one ticket holder runs actor code at a time, and tickets
 * are served in the order they were reserved. Reserving happens where the event
 * arrives (in arrival order); waiting happens on the thread that will run it.
 *
 * <p>A thread that waits on something outside the actor gives its ticket back and
 * reserves a new one afterwards ({@link Calls#outbound}), so other events run
 * meanwhile and the actor cannot deadlock on itself.
 */
final class Turn {
    static final class Ticket {
        volatile Thread thread;
    }

    private final ArrayDeque<Ticket> queue = new ArrayDeque<>();
    private volatile Ticket owner;

    /** Takes a place in line. Never blocks. */
    synchronized Ticket reserve() {
        Ticket ticket = new Ticket();
        if (owner == null) owner = ticket;
        else queue.addLast(ticket);
        return ticket;
    }

    /** Tickets waiting, not counting the owner. */
    synchronized int waiting() {
        return queue.size();
    }

    /** Blocks until {@code ticket} owns the turn. Interrupts are kept, not thrown. */
    void await(Ticket ticket) {
        ticket.thread = Thread.currentThread();
        boolean interrupted = false;
        while (owner != ticket) {
            LockSupport.park(this);
            if (Thread.interrupted()) interrupted = true;
        }
        ticket.thread = Thread.currentThread();
        if (interrupted) Thread.currentThread().interrupt();
    }

    /** Hands the turn to the next ticket. */
    void release(Ticket ticket) {
        Ticket next;
        synchronized (this) {
            if (owner != ticket) throw new IllegalStateException("released a turn it did not own");
            next = queue.pollFirst();
            owner = next;
        }
        if (next != null) {
            Thread thread = next.thread;
            if (thread != null) LockSupport.unpark(thread);
        }
    }

    boolean owns(Ticket ticket) {
        return ticket != null && owner == ticket;
    }

    /** The thread holding the turn, if it has started. */
    Thread ownerThread() {
        Ticket current = owner;
        return current == null ? null : current.thread;
    }
}
