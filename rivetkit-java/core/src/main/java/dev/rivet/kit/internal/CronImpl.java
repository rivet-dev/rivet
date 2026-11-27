package dev.rivet.kit.internal;

import dev.rivet.kit.Cron;
import dev.rivet.kit.Fn;
import java.time.Duration;
import java.time.Instant;
import java.time.ZoneId;
import java.util.ArrayList;
import java.util.List;

/** {@link Cron} over the wire. */
final class CronImpl<A> implements Cron<A> {
    private final Instance instance;

    CronImpl(Instance instance) {
        this.instance = instance;
    }

    private void set(String name, Long intervalMs, String expression, ZoneId zone, Recorder.Invocation invocation) {
        Calls.requireTurn(instance, "ctx.cron()");
        Recorder.Recorded call = Recorder.record(instance.model(), invocation);
        Native.callSync(instance.handle, Wire.OP_CRON_SET,
                new Wire.CronSetCall(name, call.action().name, intervalMs, expression,
                        zone == null ? null : zone.getId(), null),
                call.action().args.encode(call.args()));
    }

    @Override
    public void every(String name, Duration interval, Fn.Call0<A> action) {
        set(name, interval.toMillis(), null, null, p -> action.call(Recorder.as(p)));
    }

    @Override
    public <T1> void every(String name, Duration interval, Fn.Call1<A, T1> action, T1 a1) {
        set(name, interval.toMillis(), null, null, p -> action.call(Recorder.as(p), a1));
    }

    @Override
    public <T1, T2> void every(String name, Duration interval, Fn.Call2<A, T1, T2> action, T1 a1, T2 a2) {
        set(name, interval.toMillis(), null, null, p -> action.call(Recorder.as(p), a1, a2));
    }

    @Override
    public <T1, T2, T3> void every(String name, Duration interval, Fn.Call3<A, T1, T2, T3> action,
            T1 a1, T2 a2, T3 a3) {
        set(name, interval.toMillis(), null, null, p -> action.call(Recorder.as(p), a1, a2, a3));
    }

    @Override
    public void at(String name, String expression, ZoneId zone, Fn.Call0<A> action) {
        set(name, null, expression, zone, p -> action.call(Recorder.as(p)));
    }

    @Override
    public <T1> void at(String name, String expression, ZoneId zone, Fn.Call1<A, T1> action, T1 a1) {
        set(name, null, expression, zone, p -> action.call(Recorder.as(p), a1));
    }

    @Override
    public <T1, T2> void at(String name, String expression, ZoneId zone, Fn.Call2<A, T1, T2> action, T1 a1, T2 a2) {
        set(name, null, expression, zone, p -> action.call(Recorder.as(p), a1, a2));
    }

    @Override
    public <T1, T2, T3> void at(String name, String expression, ZoneId zone, Fn.Call3<A, T1, T2, T3> action,
            T1 a1, T2 a2, T3 a3) {
        set(name, null, expression, zone, p -> action.call(Recorder.as(p), a1, a2, a3));
    }

    @Override
    public boolean delete(String name) {
        Calls.requireTurn(instance, "ctx.cron().delete");
        Native.Answer answer = Native.callSync(instance.handle, Wire.OP_CRON_DELETE, new Wire.NameCall(name), null);
        return Codec.decode(answer.header(), Wire.Removed.class).removed();
    }

    @Override
    public List<Job> list() {
        Native.Answer answer = Native.callSync(instance.handle, Wire.OP_CRON_LIST, null, null);
        List<Job> out = new ArrayList<>();
        for (Wire.CronJob job : Codec.decode(answer.header(), Wire.CronList.class).jobs()) {
            out.add(new Job(job.name(), job.action(), Instant.ofEpochMilli(job.nextRunAt()),
                    job.intervalMs() == null ? null : Duration.ofMillis(job.intervalMs()), job.expression()));
        }
        return out;
    }
}
