package dev.rivet.kit.internal;

import dev.rivet.kit.Actor;
import dev.rivet.kit.ActorConn;
import dev.rivet.kit.ActorContext;
import dev.rivet.kit.Client;
import dev.rivet.kit.Cron;
import dev.rivet.kit.Db;
import dev.rivet.kit.RivetException;
import dev.rivet.kit.Schedule;
import java.util.List;
import java.util.Optional;
import java.util.concurrent.Callable;

/**
 * The {@link ActorContext} handed to an actor's constructor. The actor's API and event types
 * are only known statically to user code; here they are erased.
 */
final class ContextImpl implements ActorContext<ContextImpl.AnyActor, Object> {
    interface AnyActor extends Actor<Object, Object> {}

    private final Instance instance;
    private final ScheduleImpl<AnyActor> schedule;
    private final CronImpl<AnyActor> cron;
    private volatile Client client;

    ContextImpl(Instance instance) {
        this.instance = instance;
        this.schedule = new ScheduleImpl<>(instance);
        this.cron = new CronImpl<>(instance);
    }

    private Calls.Call turn(String what) {
        return Calls.requireTurn(instance, "ctx." + what);
    }

    @Override
    public String actorId() {
        return instance.info.actorId();
    }

    @Override
    public String name() {
        return instance.info.name();
    }

    @Override
    public List<String> key() {
        return instance.info.key();
    }

    @Override
    public String region() {
        return instance.info.region();
    }

    @Override
    public void broadcast(Object event) {
        turn("broadcast");
        Codec.EventCodec codec = instance.model().event(event.getClass());
        Native.callSync(instance.handle, Wire.OP_BROADCAST, new Wire.BroadcastCall(codec.name), codec.encode(event));
    }

    @Override
    public void send(ActorConn<?> conn, Object event) {
        turn("send");
        Codec.EventCodec codec = instance.model().event(event.getClass());
        Native.callSync(instance.handle, Wire.OP_CONN_SEND, new Wire.ConnSendCall(conn.id(), codec.name),
                codec.encode(event));
    }

    @Override
    public Optional<ActorConn<?>> conn() {
        return Optional.ofNullable(instance.currentConn());
    }

    @Override
    public List<ActorConn<?>> conns() {
        return List.copyOf(instance.conns.values());
    }

    @Override
    public Db db() {
        return instance.db;
    }

    @Override
    public Schedule<AnyActor> schedule() {
        return schedule;
    }

    @Override
    public Cron<AnyActor> cron() {
        return cron;
    }

    @Override
    public Client client() {
        Client current = client;
        if (current == null) {
            synchronized (this) {
                if (client == null) {
                    long handle = Native.handleOf(Native.callSync(instance.handle, Wire.OP_ACTOR_CLIENT, null, null));
                    client = ClientImpl.wrap(handle);
                }
                current = client;
            }
        }
        return current;
    }

    long keepAwakeRegion() {
        return Native.handleOf(Native.callSync(instance.handle, Wire.OP_KEEP_AWAKE, null, null));
    }

    @Override
    public <T> T keepAwake(Callable<T> work) {
        turn("keepAwake");
        long region = keepAwakeRegion();
        try {
            return work.call();
        } catch (RuntimeException e) {
            throw e;
        } catch (Exception e) {
            throw new RivetException("core", "internal_error", "keepAwake work failed", null, e);
        } finally {
            Native.release(region);
        }
    }

    @Override
    public void waitUntil(Runnable work) {
        turn("waitUntil");
        long region = Native.handleOf(Native.callSync(instance.handle, Wire.OP_WAIT_UNTIL, null, null));
        instance.spawn(work, region);
    }

    @Override
    public void spawn(Runnable work) {
        turn("spawn");
        instance.spawn(work, 0);
    }

    @Override
    public boolean aborted() {
        return instance.aborted;
    }

    @Override
    public <T> T blocking(Callable<T> work) {
        turn("blocking");
        return Calls.outboundChecked(work);
    }

    @Override
    public <T> T exclusive(Callable<T> work) {
        Calls.Call call = turn("exclusive");
        call.exclusive++;
        try {
            return work.call();
        } catch (RuntimeException e) {
            throw e;
        } catch (Exception e) {
            throw new RivetException("core", "internal_error", "exclusive work failed", null, e);
        } finally {
            call.exclusive--;
        }
    }

    @Override
    public void sleep() {
        Native.callSync(instance.handle, Wire.OP_SLEEP, null, null);
    }

    @Override
    public void destroy() {
        Native.callSync(instance.handle, Wire.OP_DESTROY, null, null);
    }

    void released() {
        Client current = client;
        if (current != null) current.close();
    }
}
