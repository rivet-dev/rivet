package dev.rivet.kit;

import java.time.Duration;
import java.time.Instant;
import java.time.ZoneId;
import java.util.List;

/**
 * Recurring calls to this actor's own actions ({@code A}, its API), by method reference. Setting a job
 * with an existing name replaces it, so calling these from {@link OnWake} is safe.
 */
public interface Cron<A> {
    void every(String name, Duration interval, Fn.Call0<A> action);

    <T1> void every(String name, Duration interval, Fn.Call1<A, T1> action, T1 a1);

    <T1, T2> void every(String name, Duration interval, Fn.Call2<A, T1, T2> action, T1 a1, T2 a2);

    <T1, T2, T3> void every(String name, Duration interval, Fn.Call3<A, T1, T2, T3> action, T1 a1, T2 a2, T3 a3);

    /** A cron expression ({@code "0 * * * *"}) in {@code zone} (null: UTC). */
    void at(String name, String expression, ZoneId zone, Fn.Call0<A> action);

    <T1> void at(String name, String expression, ZoneId zone, Fn.Call1<A, T1> action, T1 a1);

    <T1, T2> void at(String name, String expression, ZoneId zone, Fn.Call2<A, T1, T2> action, T1 a1, T2 a2);

    <T1, T2, T3> void at(String name, String expression, ZoneId zone, Fn.Call3<A, T1, T2, T3> action,
            T1 a1, T2 a2, T3 a3);

    boolean delete(String name);

    List<Job> list();

    record Job(String name, String action, Instant nextRunAt, Duration interval, String expression) {}
}
