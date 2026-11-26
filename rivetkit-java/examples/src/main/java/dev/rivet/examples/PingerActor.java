package dev.rivet.examples;

import dev.rivet.kit.ActorContext;
import dev.rivet.kit.Db;
import dev.rivet.kit.None;
import java.time.Duration;

public final class PingerActor implements Pinger {
    private final ActorContext<None> ctx;
    private int bumps;

    public PingerActor(ActorContext<None> ctx) {
        this.ctx = ctx;
    }

    public static void migrate(Db db) {
        db.script("CREATE TABLE IF NOT EXISTS ticks (id INTEGER PRIMARY KEY CHECK (id = 1), n INTEGER NOT NULL);"
                + "INSERT OR IGNORE INTO ticks VALUES (1, 0)");
    }

    @Override
    public int bounce(String peer, String self, int hops) {
        if (hops == 0) return 0;
        // The call gives up this actor's turn while it waits, so the peer can call back.
        return 1 + ctx.client().getOrCreate(Pinger.class, peer).bounce(self, peer, hops - 1);
    }

    @Override
    public int bump() {
        int seen = bumps;
        for (int i = 0; i < 1000; i++) Thread.onSpinWait();
        Thread.yield();
        bumps = seen + 1;
        return bumps;
    }

    @Override
    public int bumps() {
        return bumps;
    }

    @Override
    public void tickLater(long delayMillis) {
        ctx.schedule().after(Duration.ofMillis(delayMillis), Pinger::tick);
    }

    @Override
    public void tick() {
        ctx.db().exec("UPDATE ticks SET n = n + 1 WHERE id = 1");
    }

    @Override
    public long ticks() {
        return ctx.db().queryOne(Long.class, "SELECT n FROM ticks WHERE id = 1").orElse(0L);
    }

    @Override
    public long ticksConcurrently() {
        return ctx.db().queryOne(Long.class, "SELECT n FROM ticks WHERE id = 1").orElse(0L);
    }
}
