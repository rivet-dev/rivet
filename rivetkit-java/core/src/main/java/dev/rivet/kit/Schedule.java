package dev.rivet.kit;

import java.time.Duration;
import java.time.Instant;
import java.util.List;

/**
 * One-off scheduled calls to this actor's own actions, by method reference, so a
 * renamed method or wrong argument fails to compile:
 * {@code ctx.schedule().after(Duration.ofDays(30), ChatRoom::purge, cutoff)}.
 * Scheduling a method of another actor's API throws immediately. Calls survive
 * sleep and restarts.
 */
public interface Schedule {
    <A> String after(Duration delay, Fn.Call0<A> action);

    <A, T1> String after(Duration delay, Fn.Call1<A, T1> action, T1 a1);

    <A, T1, T2> String after(Duration delay, Fn.Call2<A, T1, T2> action, T1 a1, T2 a2);

    <A> String at(Instant time, Fn.Call0<A> action);

    <A, T1> String at(Instant time, Fn.Call1<A, T1> action, T1 a1);

    <A, T1, T2> String at(Instant time, Fn.Call2<A, T1, T2> action, T1 a1, T2 a2);

    boolean cancel(String id);

    List<Scheduled> list();

    record Scheduled(String id, String action, Instant runAt) {}
}
