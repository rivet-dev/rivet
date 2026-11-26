package dev.rivet.kit;

import java.time.Duration;
import java.time.Instant;
import java.time.ZoneId;
import java.util.List;

/**
 * Recurring calls to this actor's own actions, by method reference. Setting a job
 * with an existing name replaces it, so calling these from {@link OnWake} is safe.
 */
public interface Cron {
    <A> void every(String name, Duration interval, Fn.Call0<A> action);

    <A, T1> void every(String name, Duration interval, Fn.Call1<A, T1> action, T1 a1);

    /** A cron expression ({@code "0 * * * *"}) in {@code zone} (null: UTC). */
    <A> void at(String name, String expression, ZoneId zone, Fn.Call0<A> action);

    <A, T1> void at(String name, String expression, ZoneId zone, Fn.Call1<A, T1> action, T1 a1);

    boolean delete(String name);

    List<Job> list();

    record Job(String name, String action, Instant nextRunAt, Duration interval, String expression) {}
}
