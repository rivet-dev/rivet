package dev.rivet.examples;

import dev.rivet.kit.ActorConn;
import dev.rivet.kit.ActorContext;
import dev.rivet.kit.Db;
import dev.rivet.kit.OnCreate;
import dev.rivet.kit.OnDestroy;
import dev.rivet.kit.OnDisconnect;
import dev.rivet.kit.OnRequest;
import dev.rivet.kit.OnSleep;
import dev.rivet.kit.OnSubscribe;
import dev.rivet.kit.OnWake;
import dev.rivet.kit.OnWebSocket;
import dev.rivet.kit.Request;
import dev.rivet.kit.Response;
import dev.rivet.kit.UserError;
import dev.rivet.kit.WebSocket;
import java.io.InputStream;
import java.time.Duration;
import java.time.Instant;
import java.time.ZoneId;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.atomic.AtomicReference;

public final class LabActor
        implements Lab, OnCreate<Lab.CreateInput>, OnWake, OnSleep, OnDestroy, OnSubscribe<Lab.Event>,
                OnDisconnect<Lab.Params>, OnRequest, OnWebSocket {
    private final ActorContext<Lab, Event> ctx;
    private InputStream ignoredBody;
    private int disconnects;
    private int socketsClosed;
    private int spawned;
    private long lastSeenCount = -1;
    private dev.rivet.kit.Conn<Counter, Counter.NewCount> watching;

    public LabActor(ActorContext<Lab, Event> ctx) {
        this.ctx = ctx;
    }

    public static void migrate(Db db) {
        db.script("""
                CREATE TABLE IF NOT EXISTS kv (key TEXT PRIMARY KEY, value TEXT NOT NULL);
                CREATE TABLE IF NOT EXISTS facts (id INTEGER PRIMARY KEY, int_value INTEGER, float_value REAL,
                    flag INTEGER, text TEXT, blob BLOB, color TEXT, missing TEXT);
                CREATE TABLE IF NOT EXISTS accounts (name TEXT PRIMARY KEY, balance INTEGER NOT NULL);
                INSERT OR IGNORE INTO accounts VALUES ('a', 100), ('b', 0);
                CREATE TABLE IF NOT EXISTS counters (name TEXT PRIMARY KEY, n INTEGER NOT NULL);
                """);
    }

    private void count(String name) {
        ctx.db().exec("INSERT INTO counters VALUES (?, 1) ON CONFLICT(name) DO UPDATE SET n = n + 1", name);
    }

    private long counter(String name) {
        return ctx.db().queryOne(Long.class, "SELECT n FROM counters WHERE name = ?", name).orElse(0L);
    }

    // --- lifecycle ---------------------------------------------------------------------

    @Override
    public void onCreate(CreateInput input) {
        ctx.db().exec("INSERT OR REPLACE INTO kv VALUES ('created', ?)", input == null ? "none" : input.note());
    }

    @Override
    public void onWake() {
        // A background loop that stops when the actor starts shutting down.
        ctx.spawn(() -> {
            while (!ctx.aborted()) {
                try {
                    ctx.blocking(() -> {
                        Thread.sleep(25);
                        return null;
                    });
                } catch (RuntimeException interrupted) {
                    return;
                }
            }
        });
    }

    @Override
    public void onSleep() {
        count("sleeps");
    }

    @Override
    public void onDestroy() {
        count("destroys");
    }

    @Override
    public void onSubscribe(ActorConn<?> conn, Class<? extends Event> event) {
        if (event == Secret.class) throw new UserError("forbidden", "secret events are not for you");
    }

    @Override
    public void onDisconnect(ActorConn<Params> conn) {
        disconnects++;
    }

    @Override
    public Response onRequest(Request request) throws Exception {
        if (request.path().endsWith("/headers")) {
            String agent = request.header("X-Test").orElse("none");
            return Response.of(201, Map.of("x-echo", agent), (agent + "|" + request.text()).getBytes());
        }
        if (request.path().endsWith("/bytewise")) {
            // Reads one byte at a time and streams one byte at a time.
            int total = 0;
            try (InputStream body = request.body()) {
                while (body.read() >= 0) total++;
            }
            int size = total;
            return Response.streaming(200, Map.of(), out -> {
                for (int i = 0; i < size; i++) out.write('y');
            });
        }
        if (request.path().endsWith("/fails")) {
            return Response.streaming(200, Map.of(), out -> {
                out.write("partial".getBytes());
                out.flush();
                throw new IllegalStateException("writer failed");
            });
        }
        if (request.path().endsWith("/ignored")) {
            // Answers without reading the body; the SDK must still close it.
            ignoredBody = request.body();
        }
        return Response.text(404, "nope");
    }

    @Override
    public void onWebSocket(WebSocket socket, ActorConn<?> conn, Optional<Request> request) {
        socket.onMessage(message -> {
            if (!message.binary() && message.text().equals("bye")) {
                socket.close(4000, "requested");
            } else if (!message.binary() && message.text().equals("hold")) {
                // Holds the turn so later frames pile up (overload test).
                try {
                    Thread.sleep(300);
                } catch (InterruptedException e) {
                    Thread.currentThread().interrupt();
                }
            } else if (!message.binary() && message.text().equals("quiet")) {
                // no reply
            } else if (message.binary()) {
                socket.send(message.data());
            } else {
                socket.send("echo:" + message.text());
            }
        });
        socket.onClose(close -> socketsClosed++);
    }

    // --- actions -----------------------------------------------------------------------

    @Override
    public List<String> info() {
        return List.of(ctx.actorId(), ctx.name(), String.join("/", ctx.key()), ctx.region());
    }

    @Override
    public Row sqlTypes() {
        ctx.db().exec("DELETE FROM facts");
        ctx.db().exec("INSERT INTO facts (int_value, float_value, flag, text, blob, color, missing) VALUES (?, ?, ?, ?, ?, ?, ?)",
                (short) 7, 2.5d, true, "hi", new byte[] {1, 2, 3}, Color.GREEN, null);
        ctx.db().exec("INSERT INTO facts (int_value, text) VALUES (?, ?)", (byte) 1, "other");
        ctx.db().exec("INSERT INTO facts (int_value, float_value, text) VALUES (?, ?, ?)", 2, 1.5f, "other");
        return ctx.db().queryOne(Row.class, "SELECT int_value, float_value, flag, text, blob, color, missing FROM facts WHERE text = ?", "hi")
                .orElseThrow();
    }

    @Override
    public long rowsMatching(String label) {
        List<String> texts = ctx.db().query(String.class, "SELECT text FROM facts WHERE text = ?", label);
        Optional<Long> none = ctx.db().queryOne(Long.class, "SELECT int_value FROM facts WHERE text = 'absent'");
        if (none.isPresent()) throw new IllegalStateException("expected no row");
        return texts.size();
    }

    @Override
    public void badParam() {
        ctx.db().exec("SELECT ?", new Object());
    }

    @Override
    public long scalarFromTwoColumns() {
        return ctx.db().queryOne(Long.class, "SELECT 1, 2").orElseThrow();
    }

    @Override
    public long transfer(boolean fail) {
        try {
            ctx.db().transaction(tx -> {
                tx.exec("UPDATE accounts SET balance = balance - 10 WHERE name = 'a'");
                tx.exec("UPDATE accounts SET balance = balance + 10 WHERE name = 'b'");
                if (fail) throw new IllegalStateException("rolled back");
                return null;
            });
        } catch (IllegalStateException rolledBack) {
            // the transaction rolled back
        }
        return ctx.db().queryOne(Long.class, "SELECT balance FROM accounts WHERE name = 'b'").orElseThrow();
    }

    @Override
    public List<Long> batch() {
        List<Db.Exec> results = ctx.db().batch(List.of(
                Db.Statement.of("INSERT OR REPLACE INTO kv VALUES (?, ?)", "b1", "x"),
                Db.Statement.of("INSERT OR REPLACE INTO kv VALUES (?, ?)", "b2", "y")));
        List<Long> changes = new ArrayList<>();
        for (Db.Exec result : results) changes.add(result.changes());
        return changes;
    }

    @Override
    public int schedules() {
        String later = ctx.schedule().at(Instant.now().plus(Duration.ofHours(1)), Lab::store, "at", "1");
        ctx.schedule().after(Duration.ofHours(2), Lab::store, "after", "2");
        ctx.schedule().at(Instant.now().plus(Duration.ofHours(3)), Lab::sleepNow);
        ctx.schedule().at(Instant.now().plus(Duration.ofHours(3)), Lab::store, "a", "b");
        ctx.schedule().at(Instant.now().plus(Duration.ofHours(3)), Lab::stored, "k");
        ctx.schedule().after(Duration.ofHours(4), Lab::stored, "k");
        int before = ctx.schedule().list().size();
        if (!ctx.schedule().cancel(later)) throw new IllegalStateException("cancel failed");
        return before * 10 + ctx.schedule().list().size();
    }

    @Override
    public List<String> crons() {
        ctx.cron().every("tick", Duration.ofMinutes(5), Lab::sleeps);
        ctx.cron().every("store", Duration.ofMinutes(5), Lab::stored, "k");
        ctx.cron().at("nightly", "0 3 * * *", ZoneId.of("Europe/Prague"), Lab::sleeps);
        ctx.cron().at("hourly", "0 * * * *", null, Lab::stored, "k");
        List<String> names = new ArrayList<>();
        for (var job : ctx.cron().list()) names.add(job.name());
        if (!ctx.cron().delete("tick")) throw new IllegalStateException("delete failed");
        names.add("left:" + ctx.cron().list().size());
        return names;
    }

    @Override
    public String keepAwake() {
        return ctx.keepAwake(() -> "awake");
    }

    @Override
    public int spawnAndWait() throws Exception {
        CompletableFuture<Void> done = new CompletableFuture<>();
        ctx.waitUntil(() -> {
            spawned++;
            done.complete(null);
        });
        // The task runs as a turn after this one; wait without holding the turn.
        ctx.blocking(() -> done.get());
        return spawned;
    }

    @Override
    public String blocking() {
        return ctx.blocking(() -> {
            Thread.sleep(5);
            return "slept";
        });
    }

    @Override
    public String exclusive() {
        return ctx.exclusive(() -> ctx.blocking(() -> "held"));
    }

    @Override
    public String touchCtxFromForeignThread() throws Exception {
        AtomicReference<String> error = new AtomicReference<>("none");
        Thread thread = new Thread(() -> {
            try {
                ctx.broadcast(new Pushed("nope"));
            } catch (IllegalStateException e) {
                error.set(e.getMessage());
            }
        });
        thread.start();
        thread.join();
        return error.get();
    }

    @Override
    public int pushToCaller(String text) {
        ActorConn<?> conn = ctx.conn().orElseThrow(() -> new UserError("no_conn", "call over a connection"));
        ctx.send(conn, new Pushed(text));
        return ctx.conns().size();
    }

    @Override
    public void kickCaller() {
        ctx.conn().orElseThrow().disconnect("kicked");
    }

    @Override
    public int disconnects() {
        return disconnects;
    }

    @Override
    public boolean ignoredBodyClosed() throws Exception {
        InputStream body = ignoredBody;
        if (body == null) return false;
        // The body is 1 MiB, so an open one returns data (or blocks waiting for it). A closed one
        // ends at once (-1, as a closed SequenceInputStream does) or throws.
        CompletableFuture<Boolean> closed = new CompletableFuture<>();
        Thread.ofVirtual().start(() -> {
            try {
                closed.complete(body.read() < 0);
            } catch (java.io.IOException e) {
                closed.complete(true);
            }
        });
        try {
            return closed.get(1, java.util.concurrent.TimeUnit.SECONDS);
        } catch (java.util.concurrent.TimeoutException stillOpen) {
            return false;
        }
    }

    @Override
    @SuppressWarnings("unchecked")
    public String callerUser() {
        ActorConn<Params> conn = (ActorConn<Params>) ctx.conn().orElseThrow(() -> new UserError("no_conn", "call over a connection"));
        return conn.params().user();
    }

    @Override
    public String createdWith() {
        return stored("created");
    }

    @Override
    public long sleeps() {
        return counter("sleeps");
    }

    @Override
    public void sleepNow() {
        ctx.sleep();
    }

    @Override
    public void destroyNow() {
        ctx.destroy();
    }

    @Override
    public void userErrorWithMetadata() {
        throw new UserError("bad_input", "field is wrong", Map.of("field", "name"));
    }

    @Override
    public void crash() {
        throw new IllegalStateException("boom");
    }

    @Override
    public long add2(long a, long b) {
        return a + b;
    }

    @Override
    public long add3(long a, long b, long c) {
        return a + b + c;
    }

    @Override
    public void store(String key, String value) {
        ctx.db().exec("INSERT OR REPLACE INTO kv VALUES (?, ?)", key, value);
    }

    @Override
    public void store3(String key, String value, String suffix) {
        store(key, value + suffix);
    }

    @Override
    public String stored(String key) {
        return ctx.db().queryOne(String.class, "SELECT value FROM kv WHERE key = ?", key).orElse(null);
    }

    @Override
    public void watchCounter(String counterKey) {
        watching = ctx.client().connect(Counter.class, counterKey);
        watching.on(Counter.NewCount.class, event -> lastSeenCount = event.count());
    }

    @Override
    public long lastSeenCount() {
        return lastSeenCount;
    }

    @Override
    public int socketsClosed() {
        return socketsClosed;
    }
}
