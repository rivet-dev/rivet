package dev.rivet.kit.internal;

import dev.rivet.kit.ActorContext;
import dev.rivet.kit.Db;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.AtomicLong;
import java.util.function.Consumer;
import java.util.function.Function;

/** One registered actor type: its API model, how to build instances, and the live instances. */
public final class ActorType implements Native.TypeSink {
    private static final AtomicLong NEXT_ID = new AtomicLong(1);

    final long id = NEXT_ID.getAndIncrement();
    final ApiModel model;
    final Function<ActorContext<?, ?>, ?> factory;
    final Consumer<Db> migrate;
    /** Events an instance lets wait for its turn before refusing more. */
    final int maxWaiting;
    private final Map<Long, Instance> instances = new ConcurrentHashMap<>();

    public ActorType(ApiModel model, Function<ActorContext<?, ?>, ?> factory, Consumer<Db> migrate, int maxWaiting) {
        this.model = model;
        this.factory = factory;
        this.migrate = migrate;
        this.maxWaiting = maxWaiting;
    }

    public long id() {
        return id;
    }

    public ApiModel model() {
        return model;
    }

    /**
     * Hooks reported to Rust. The actor class is only known once the factory runs,
     * so every instance hook is declared; an absent one is answered right away.
     */
    public Wire.Hooks hooks() {
        // Every hook but migrate: the instance decides (see Wire.Hooks).
        return new Wire.Hooks(migrate != null, true, true, true, true, true, true, true, true, true, true, true, true);
    }

    @Override
    public void onEvent(int kind, long actor, long event, byte[] header, byte[] body) {
        if (kind == Wire.RELEASED) {
            Instance instance = instances.remove(actor);
            if (instance != null) instance.released();
            return;
        }
        Instance instance = instances.computeIfAbsent(actor, handle -> new Instance(this, handle));
        if (kind == Wire.CANCEL) {
            instance.cancel(Codec.decode(header, Wire.CancelEvent.class).event());
        } else if (kind == Wire.ABORT) {
            instance.abort();
        } else {
            instance.submit(kind, event, header, body);
        }
    }
}
