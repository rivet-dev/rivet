package dev.rivet.kit.internal;

import dev.rivet.kit.Fn;
import dev.rivet.kit.Schedule;
import java.time.Duration;
import java.time.Instant;
import java.util.ArrayList;
import java.util.List;

/** {@link Schedule} over the wire; method references become action names and CBOR args. */
final class ScheduleImpl implements Schedule {
    private final Instance instance;

    ScheduleImpl(Instance instance) {
        this.instance = instance;
    }

    @SuppressWarnings("unchecked")
    static <A> A as(Object proxy) {
        return (A) proxy;
    }

    private String schedule(Long delayMs, Long timestampMs, Recorder.Invocation invocation) {
        Calls.requireTurn(instance, "ctx.schedule()");
        Recorder.Recorded call = Recorder.record(instance.model(), invocation);
        Native.Answer answer = Native.callSync(instance.handle, Wire.OP_SCHEDULE,
                new Wire.ScheduleCall(call.action().name, delayMs, timestampMs), call.action().args.encode(call.args()));
        return Codec.decode(answer.header(), Wire.ScheduleId.class).id();
    }

    private static long millis(Duration delay) {
        if (delay.isNegative()) throw new IllegalArgumentException("delay must not be negative");
        return delay.toMillis();
    }

    @Override
    public <A> String after(Duration delay, Fn.Call0<A> action) {
        return schedule(millis(delay), null, p -> action.call(as(p)));
    }

    @Override
    public <A, T1> String after(Duration delay, Fn.Call1<A, T1> action, T1 a1) {
        return schedule(millis(delay), null, p -> action.call(as(p), a1));
    }

    @Override
    public <A, T1, T2> String after(Duration delay, Fn.Call2<A, T1, T2> action, T1 a1, T2 a2) {
        return schedule(millis(delay), null, p -> action.call(as(p), a1, a2));
    }

    @Override
    public <A> String at(Instant time, Fn.Call0<A> action) {
        return schedule(null, time.toEpochMilli(), p -> action.call(as(p)));
    }

    @Override
    public <A, T1> String at(Instant time, Fn.Call1<A, T1> action, T1 a1) {
        return schedule(null, time.toEpochMilli(), p -> action.call(as(p), a1));
    }

    @Override
    public <A, T1, T2> String at(Instant time, Fn.Call2<A, T1, T2> action, T1 a1, T2 a2) {
        return schedule(null, time.toEpochMilli(), p -> action.call(as(p), a1, a2));
    }

    @Override
    public boolean cancel(String id) {
        Calls.requireTurn(instance, "ctx.schedule().cancel");
        Native.Answer answer = Native.callSync(instance.handle, Wire.OP_SCHEDULE_CANCEL, new Wire.ScheduleId(id), null);
        return Codec.decode(answer.header(), Wire.Removed.class).removed();
    }

    @Override
    public List<Scheduled> list() {
        Native.Answer answer = Native.callSync(instance.handle, Wire.OP_SCHEDULE_LIST, null, null);
        List<Scheduled> out = new ArrayList<>();
        for (Wire.ScheduledEvent event : Codec.decode(answer.header(), Wire.ScheduleList.class).events()) {
            out.add(new Scheduled(event.id(), event.action(), Instant.ofEpochMilli(event.runAt())));
        }
        return out;
    }
}
