package dev.rivet.examples;

import dev.rivet.kit.ActorConn;
import dev.rivet.kit.ActorContext;
import dev.rivet.kit.Db;
import dev.rivet.kit.OnBeforeConnect;
import dev.rivet.kit.OnConnect;
import dev.rivet.kit.OnDisconnect;
import dev.rivet.kit.OnWake;
import dev.rivet.kit.Request;
import dev.rivet.kit.UserError;
import java.time.Duration;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;

public final class ChatRoomActor
        implements ChatRoom, OnBeforeConnect<ChatRoom.Params>, OnConnect<ChatRoom.Params>, OnDisconnect<ChatRoom.Params>,
                OnWake {
    private final ActorContext<ChatRoom, Event> ctx;
    // Vars: plain fields, rebuilt on every wake. Turns make them safe without locks.
    private final Map<String, String> userByConn = new HashMap<>();
    private int sentSinceWake;

    public ChatRoomActor(ActorContext<ChatRoom, Event> ctx) {
        this.ctx = ctx;
    }

    public static void migrate(Db db) {
        db.script("CREATE TABLE IF NOT EXISTS messages (id INTEGER PRIMARY KEY, sender TEXT NOT NULL, text TEXT NOT NULL, at INTEGER NOT NULL)");
    }

    @Override
    public void onWake() {
        // Replaces the job if it exists, so this is safe on every wake.
        ctx.cron().every("purge", Duration.ofHours(1), ChatRoom::purgeOlderThan, 0L);
    }

    // Runs for every connection, including the short-lived one behind each stateless
    // call (`client.getOrCreate(...).send(...)`), which carries no params.
    @Override
    public void onBeforeConnect(Params params, Optional<Request> request) {
        if (params == null) return;
        if (params.user() == null || params.user().isBlank()) {
            throw new UserError("unauthorized", "connect with a user name");
        }
        if ("banned".equals(params.token())) throw new UserError("forbidden", "this token is banned");
    }

    @Override
    public void onConnect(ActorConn<Params> conn) {
        if (conn.params() == null) return;
        userByConn.put(conn.id(), conn.params().user());
        ctx.broadcast(new Joined(conn.params().user()));
    }

    @Override
    public void onDisconnect(ActorConn<Params> conn) {
        userByConn.remove(conn.id());
    }

    @Override
    public Message send(String text) {
        if (text == null || text.isBlank()) throw new UserError("empty", "text must not be blank");
        String sender = ctx.conn().map(conn -> userByConn.get(conn.id())).orElse("anonymous");
        Message message = ctx.db()
                .queryOne(Message.class,
                        "INSERT INTO messages (sender, text, at) VALUES (?, ?, ?) RETURNING sender, text, at",
                        sender, text, System.currentTimeMillis())
                .orElseThrow();
        sentSinceWake++;
        ctx.broadcast(new NewMessage(message));
        return message;
    }

    @Override
    public List<Message> history(int limit) {
        List<Message> newestFirst = ctx.db()
                .query(Message.class, "SELECT sender, text, at FROM messages ORDER BY id DESC LIMIT ?", limit);
        List<Message> out = new ArrayList<>(newestFirst);
        java.util.Collections.reverse(out);
        return out;
    }

    @Override
    public int sentSinceWake() {
        return sentSinceWake;
    }

    @Override
    public List<String> online() {
        return List.copyOf(userByConn.values());
    }

    @Override
    public int purgeOlderThan(long cutoffMillis) {
        long cutoff = cutoffMillis > 0 ? cutoffMillis : System.currentTimeMillis() - Duration.ofDays(30).toMillis();
        return (int) ctx.db().exec("DELETE FROM messages WHERE at < ?", cutoff).changes();
    }

    @Override
    public void goToSleep() {
        ctx.sleep();
    }
}
