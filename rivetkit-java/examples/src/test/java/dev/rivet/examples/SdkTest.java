package dev.rivet.examples;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import dev.rivet.kit.Client;
import dev.rivet.kit.Conn;
import dev.rivet.kit.Handle;
import dev.rivet.kit.Response;
import dev.rivet.kit.RivetException;
import dev.rivet.kit.WebSocket;
import dev.rivet.kit.internal.Native;
import dev.rivet.kit.testing.RivetTest;
import java.time.Duration;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.concurrent.BlockingQueue;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.LinkedBlockingQueue;
import java.util.concurrent.TimeUnit;
import java.util.function.BooleanSupplier;
import org.junit.jupiter.api.AfterAll;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;

/** End to end: the example actors, served by the Rust core against a local engine. */
class SdkTest {
    private static RivetTest rivet;
    private static Client client;

    @BeforeAll
    static void start() {
        rivet = RivetTest.start(Examples.registry());
        client = rivet.client();
    }

    @AfterAll
    static void stop() throws InterruptedException {
        if (rivet != null) rivet.close();
        for (int i = 0; i < 100 && Native.pendingCount() > 0; i++) Thread.sleep(50);
        assertEquals(0, Native.pendingCount(), "calls left waiting on Rust");
    }

    private static void eventually(BooleanSupplier condition, Duration timeout) throws InterruptedException {
        long deadline = System.nanoTime() + timeout.toNanos();
        while (!condition.getAsBoolean()) {
            if (System.nanoTime() > deadline) throw new AssertionError("condition not met within " + timeout);
            Thread.sleep(100);
        }
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
        try (Conn<ChatRoom, ChatRoom.Event> conn =
                client.connect(ChatRoom.class, () -> new ChatRoom.Params("alice", "t"), room)) {
            conn.onEvent(events::add);
            ChatRoom.Message sent = conn.api().send("hello");
            assertEquals("alice", sent.sender());

            List<ChatRoom.Event> seen = new ArrayList<>();
            while (seen.stream().noneMatch(e -> e instanceof ChatRoom.NewMessage)) {
                ChatRoom.Event event = events.poll(10, TimeUnit.SECONDS);
                if (event == null) throw new AssertionError("no event; saw " + seen);
                seen.add(event);
            }
            ChatRoom.NewMessage message = (ChatRoom.NewMessage) seen.stream()
                    .filter(e -> e instanceof ChatRoom.NewMessage)
                    .findFirst()
                    .orElseThrow();
            assertEquals("hello", message.message().text());
            assertEquals(List.of("alice"), conn.api().online());
        }
        ChatRoom chat = client.getOrCreate(ChatRoom.class, room);
        assertEquals(List.of("hello"), chat.history(10).stream().map(ChatRoom.Message::text).toList());
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
        }, Duration.ofSeconds(30));
        assertEquals(List.of("one"), chat.history(10).stream().map(ChatRoom.Message::text).toList());
    }

    @Test
    void httpBodiesStreamBothWays() {
        Handle<Files> files = client.handle(Files.class, RivetTest.uniqueKey());
        rivet.retry(() -> files.api().uploadedBytes());
        byte[] upload = new byte[1024 * 1024 + 7];
        Response uploaded = files.fetch("POST", "/upload", Map.of("content-type", "application/octet-stream"), upload);
        assertEquals(200, uploaded.status());
        assertEquals(Integer.toString(upload.length), uploaded.text());
        assertEquals(upload.length, files.api().uploadedBytes());

        Response download = files.fetch("GET", "/download?n=3000000", Map.of(), null);
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
        eventually(() -> pinger.ticks() == 1, Duration.ofSeconds(20));
        assertEquals(1, pinger.ticksConcurrently());
    }
}
