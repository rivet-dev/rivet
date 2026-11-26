package dev.rivet.examples;

import dev.rivet.kit.ActorContext;
import dev.rivet.kit.Db;
import dev.rivet.kit.UserError;

public final class CounterActor implements Counter {
    private final ActorContext<NewCount> ctx;

    public CounterActor(ActorContext<NewCount> ctx) {
        this.ctx = ctx;
    }

    /** Runs before the constructor on every start. */
    public static void migrate(Db db) {
        db.script("""
                CREATE TABLE IF NOT EXISTS counter (id INTEGER PRIMARY KEY CHECK (id = 1), count INTEGER NOT NULL);
                INSERT OR IGNORE INTO counter VALUES (1, 0);
                """);
    }

    @Override
    public long increment(long amount) {
        if (amount < 0) throw new UserError("invalid_amount", "amount must be >= 0");
        long count = ctx.db()
                .queryOne(Long.class, "UPDATE counter SET count = count + ? WHERE id = 1 RETURNING count", amount)
                .orElseThrow();
        ctx.broadcast(new NewCount(count));
        return count;
    }

    @Override
    public long get() {
        return ctx.db().queryOne(Long.class, "SELECT count FROM counter WHERE id = 1").orElse(0L);
    }
}
