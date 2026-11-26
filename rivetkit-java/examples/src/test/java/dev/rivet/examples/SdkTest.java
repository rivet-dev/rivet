package dev.rivet.examples;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import dev.rivet.kit.Client;
import dev.rivet.kit.Conn;
import dev.rivet.kit.FetchResponse;
import dev.rivet.kit.Handle;
import dev.rivet.kit.RivetException;
import dev.rivet.kit.WebSocket;
import dev.rivet.kit.testing.RivetTest;
import java.time.Duration;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Map;
import java.util.concurrent.BlockingQueue;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.LinkedBlockingQueue;
import java.util.concurrent.TimeUnit;
import java.util.function.BooleanSupplier;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;

/** End to end: the example actors, served by the Rust core against a local engine. */
@ExtendWith(TestRivet.class)
class SdkTest {
    private static RivetTest rivet;
    private static Client client;

    @BeforeAll
    static void start() {
        rivet = TestRivet.get();
        client = rivet.client();
    }

    private static void eventually(BooleanSupplier condition, Duration timeout) throws InterruptedException {
        TestRivet.eventually(condition, timeout);
    }

    @Test
    void actionsAreTyped() {
        Counter counter = client.getOrCreate(Counter.class, RivetTest.uniqueKey());
        assertEquals(5, rivet.retry(() -> counter.increment(5)));
        assertEquals(8, counter.increment(3));
        assertEquals(8, counter.get());
    }

    @Test
    void userErrorsReachTheClient() {
        Counter counter = client.getOrCreate(Counter.class, RivetTest.uniqueKey());
        rivet.retry(counter::get);
        RivetException e = assertThrows(RivetException.class, () -> counter.increment(-1));
        assertEquals("user", e.group());
        assertEquals("invalid_amount", e.code());
        assertEquals("amount must be >= 0", e.detail());
    }

    @Test
    void asyncCallsByMethodReference() throws Exception {
        Handle<Counter> handle = client.handle(Counter.class, RivetTest.uniqueKey());
        rivet.retry(() -> handle.api().get());
        assertEquals(2L, handle.async(Counter::increment, 2L).get(10, TimeUnit.SECONDS));
        assertEquals(2L, handle.async(Counter::get).get(10, TimeUnit.SECONDS));
    }

    @Test
    void connectionsDeliverTypedEvents() throws Exception {
        String room = RivetTest.uniqueKey();
        rivet.retry(() -> client.getOrCreate(ChatRoom.class, room).history(1));
        BlockingQueue<ChatRoom.Event> events = new LinkedBlockingQueue<>();
        int sends = 0;
        try (Conn<ChatRoom, ChatRoom.Event> conn =
                client.connect(ChatRoom.class, () -> new ChatRoom.Params("alice", "t"), room)) {
            conn.onEvent(events::add);
            eventually(() -> conn.status() == Conn.Status.CONNECTED, Duration.ofSeconds(10));

            // Known race in the shared Rust client (rivetkit-client connection.rs): a subscribe
            // sent while the connection is opening can be overtaken by the first action, so its
            // broadcast is missed. Retry the send a few times; the event must arrive eventually.
            // TODO: drop the retry once the client orders subscriptions before queued actions.
            List<ChatRoom.Event> seen = new ArrayList<>();
            while (seen.stream().noneMatch(e -> e instanceof ChatRoom.NewMessage)) {
                if (sends == 3) throw new AssertionError("no event after " + sends + " sends; saw " + seen);
                ChatRoom.Message sent = conn.api().send("hello");
                sends++;
                assertEquals("alice", sent.sender());
                for (ChatRoom.Event event; (event = events.poll(3, TimeUnit.SECONDS)) != null; ) {
                    seen.add(event);
                    if (event instanceof ChatRoom.NewMessage) break;
                }
            }
            ChatRoom.NewMessage message = (ChatRoom.NewMessage) seen.stream()
                    .filter(e -> e instanceof ChatRoom.NewMessage)
                    .findFirst()
                    .orElseThrow();
            assertEquals("hello", message.message().text());
            assertEquals(List.of("alice"), conn.api().online());
        }
        ChatRoom chat = client.getOrCreate(ChatRoom.class, room);
        assertEquals(Collections.nCopies(sends, "hello"),
                chat.history(10).stream().map(ChatRoom.Message::text).toList());
    }

    @Test
    void beforeConnectCanReject() throws Exception {
        String room = RivetTest.uniqueKey();
        rivet.retry(() -> client.getOrCreate(ChatRoom.class, room).history(1));
        BlockingQueue<RivetException> errors = new LinkedBlockingQueue<>();
        try (Conn<ChatRoom, ChatRoom.Event> conn =
                client.connect(ChatRoom.class, () -> new ChatRoom.Params("mallory", "banned"), room)) {
            conn.onError(errors::add);
            conn.api(); // connecting starts on open; the rejection arrives as an error
            RivetException error = errors.poll(20, TimeUnit.SECONDS);
            if (error == null) throw new AssertionError("no connection error");
            assertTrue(error.getMessage().contains("banned"), error.getMessage());
        }
    }

    @Test
    void varsResetOnWakeButSqlPersists() throws Exception {
        String room = RivetTest.uniqueKey();
        ChatRoom chat = client.getOrCreate(ChatRoom.class, room);
        rivet.retry(() -> chat.send("one"));
        assertEquals(1, chat.sentSinceWake());
        chat.goToSleep();
        eventually(() -> {
            try {
                return chat.sentSinceWake() == 0;
            } catch (RivetException e) {
                return false;
            }
        }, Duration.ofSeconds(15));
        assertEquals(List.of("one"), chat.history(10).stream().map(ChatRoom.Message::text).toList());
    }

    @Test
    void httpBodiesStreamBothWays() {
        Handle<Files> files = client.handle(Files.class, RivetTest.uniqueKey());
        rivet.retry(() -> files.api().uploadedBytes());
        byte[] upload = new byte[1024 * 1024 + 7];
        FetchResponse uploaded = files.fetch("POST", "/upload", Map.of("content-type", "application/octet-stream"), upload);
        assertEquals(200, uploaded.status());
        assertEquals(Integer.toString(upload.length), uploaded.text());
        assertEquals(upload.length, files.api().uploadedBytes());

        FetchResponse download = files.fetch("GET", "/download?n=3000000", Map.of(), null);
        assertEquals(200, download.status());
        assertEquals(3_000_000, download.bytes().length);
    }

    @Test
    void websocketsEcho() throws Exception {
        Handle<Echo> echo = client.handle(Echo.class, RivetTest.uniqueKey());
        rivet.retry(() -> echo.api().frames());
        BlockingQueue<String> received = new LinkedBlockingQueue<>();
        try (WebSocket socket = echo.webSocket("/")) {
            socket.onMessage(message -> received.add(message.text()));
            socket.send("a");
            socket.send("b");
            assertEquals("1:a", received.poll(10, TimeUnit.SECONDS));
            assertEquals("2:b", received.poll(10, TimeUnit.SECONDS));
        }
        assertEquals(2, echo.api().frames());
    }

    @Test
    void websocketBurstsArriveInOrder() throws Exception {
        // More frames than the client's send queue holds: sends wait for room instead of
        // buffering without bound, and nothing is lost or reordered.
        Handle<Echo> echo = client.handle(Echo.class, RivetTest.uniqueKey());
        rivet.retry(() -> echo.api().frames());
        int n = 500;
        BlockingQueue<String> received = new LinkedBlockingQueue<>();
        try (WebSocket socket = echo.webSocket("/")) {
            socket.onMessage(message -> received.add(message.text()));
            for (int i = 1; i <= n; i++) socket.send("m" + i);
            for (int i = 1; i <= n; i++) assertEquals(i + ":m" + i, received.poll(10, TimeUnit.SECONDS));
        }
    }

    @Test
    void turnsMakeFieldUpdatesAtomic() throws Exception {
        Handle<Pinger> pinger = client.handle(Pinger.class, RivetTest.uniqueKey());
        rivet.retry(() -> pinger.api().bumps());
        List<CompletableFuture<Integer>> calls = new ArrayList<>();
        for (int i = 0; i < 100; i++) calls.add(pinger.async(Pinger::bump));
        CompletableFuture.allOf(calls.toArray(CompletableFuture[]::new)).get(60, TimeUnit.SECONDS);
        assertEquals(100, pinger.api().bumps());
    }

    @Test
    void actorsCanCallEachOtherBack() {
        String a = RivetTest.uniqueKey();
        String b = RivetTest.uniqueKey();
        Pinger pinger = client.getOrCreate(Pinger.class, a);
        rivet.retry(() -> client.getOrCreate(Pinger.class, b).bumps());
        assertEquals(4, rivet.retry(() -> pinger.bounce(b, a, 4)));
    }

    @Test
    void schedulesFire() throws Exception {
        Pinger pinger = client.getOrCreate(Pinger.class, RivetTest.uniqueKey());
        rivet.retry(pinger::ticks);
        pinger.tickLater(200);
        eventually(() -> pinger.ticks() == 1, Duration.ofSeconds(15));
        assertEquals(1, pinger.ticksConcurrently());
    }
}
