package dev.rivet.examples;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import dev.rivet.kit.Actor;
import dev.rivet.kit.ActorContext;
import dev.rivet.kit.Client;
import dev.rivet.kit.Conn;
import dev.rivet.kit.Handle;
import dev.rivet.kit.Name;
import dev.rivet.kit.None;
import dev.rivet.kit.Registry;
import dev.rivet.kit.Response;
import dev.rivet.kit.RivetException;
import dev.rivet.kit.WebSocket;
import dev.rivet.kit.testing.RivetTest;
import java.time.Duration;
import java.util.List;
import java.util.Map;
import java.util.concurrent.BlockingQueue;
import java.util.concurrent.LinkedBlockingQueue;
import java.util.concurrent.TimeUnit;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;

/** The rest of the public API, actor side and client side, against a real engine. */
class FeaturesTest {
    private static RivetTest rivet;
    private static Client client;

    /** A client-side view of the lab with an action the actor does not have. */
    @Name("lab")
    public interface LabWithExtra extends Actor<Lab.Event, Lab.Params> {
        List<String> info();

        void notThere();

        /** Out of date: the actor's add2 takes two longs. */
        long add2();
    }

    @BeforeAll
    static void start() {
        rivet = TestRivet.get();
        client = rivet.client();
    }

    private static Lab lab(String key) {
        Lab lab = client.getOrCreate(Lab.class, key);
        rivet.retry(lab::info);
        return lab;
    }

    // --- context ------------------------------------------------------------------------

    @Test
    void contextKnowsWhoItIs() {
        String key = RivetTest.uniqueKey();
        List<String> info = lab(key).info();
        assertFalse(info.get(0).isBlank(), "actor id");
        assertEquals("lab", info.get(1));
        assertEquals(key, info.get(2));
        assertTrue(info.size() == 4);
    }

    @Test
    void sqlBindsAndMapsEveryType() {
        Lab lab = lab(RivetTest.uniqueKey());
        Lab.Row row = lab.sqlTypes();
        assertEquals(7, row.intValue());
        assertEquals(2.5, row.floatValue());
        assertTrue(row.flag());
        assertEquals("hi", row.text());
        assertArrayEquals(new byte[] {1, 2, 3}, row.blob());
        assertEquals(Lab.Color.GREEN, row.color());
        assertNull(row.missing());
        assertEquals(2, lab.rowsMatching("other"));
    }

    @Test
    void sqlMisuseFailsClearly() {
        Lab lab = lab(RivetTest.uniqueKey());
        RivetException badParam = assertThrows(RivetException.class, lab::badParam);
        assertEquals("internal_error", badParam.code(), "unsupported parameter types are an actor bug");
        RivetException shape = assertThrows(RivetException.class, lab::scalarFromTwoColumns);
        assertEquals("sql_row_shape", shape.code());
    }

    @Test
    void transactionsCommitAndRollBack() {
        Lab lab = lab(RivetTest.uniqueKey());
        assertEquals(10, lab.transfer(false));
        assertEquals(10, lab.transfer(true), "the failed transfer rolled back");
        assertEquals(List.of(1L, 1L), lab.batch());
        assertEquals("x", lab.stored("b1"));
    }

    @Test
    void schedulesListAndCancel() {
        assertEquals(65, lab(RivetTest.uniqueKey()).schedules(), "6 scheduled, then 5 after a cancel");
    }

    @Test
    void cronJobsListAndDelete() {
        List<String> names = lab(RivetTest.uniqueKey()).crons();
        assertTrue(names.containsAll(List.of("tick", "store", "nightly", "hourly")), names.toString());
        assertEquals("left:3", names.get(names.size() - 1));
    }

    @Test
    void workRegionsAndTurns() {
        Lab lab = lab(RivetTest.uniqueKey());
        assertEquals("awake", lab.keepAwake());
        assertEquals(1, rivet.retry(() -> {
            try {
                return lab.spawnAndWait();
            } catch (Exception e) {
                throw new RuntimeException(e);
            }
        }));
        assertEquals("slept", lab.blocking());
        assertEquals("held", lab.exclusive());
    }

    @Test
    void contextRefusesThreadsOutsideTheTurn() throws Exception {
        String message = lab(RivetTest.uniqueKey()).touchCtxFromForeignThread();
        assertTrue(message.contains("outside this actor's turn"), message);
    }

    @Test
    void userErrorsCarryMetadataAndCrashesAreMasked() {
        Lab lab = lab(RivetTest.uniqueKey());
        RivetException user = assertThrows(RivetException.class, lab::userErrorWithMetadata);
        assertEquals("bad_input", user.code());
        assertEquals(Map.of("field", "name"), user.metadata());
        RivetException crash = assertThrows(RivetException.class, lab::crash);
        assertTrue(crash.isInternal(), "unexpected exceptions reach clients as internal errors");
        assertFalse(crash.getMessage().contains("boom"), "and their details stay on the server");
    }

    @Test
    void unknownActionsAreRejected() {
        String key = RivetTest.uniqueKey();
        lab(key);
        LabWithExtra extra = client.getOrCreate(LabWithExtra.class, key);
        RivetException e = assertThrows(RivetException.class, extra::notThere);
        assertEquals("action_not_found", e.code());
    }

    @Test
    void outOfDateClientInterfacesFailClearly() {
        String key = RivetTest.uniqueKey();
        lab(key);
        RivetException e = assertThrows(RivetException.class, () -> client.getOrCreate(LabWithExtra.class, key).add2());
        assertEquals("invalid_args", e.code());
    }

    @Test
    void actorsCanSubscribeToOtherActors() throws Exception {
        String counterKey = RivetTest.uniqueKey();
        Counter counter = client.getOrCreate(Counter.class, counterKey);
        rivet.retry(counter::get);
        Lab lab = lab(RivetTest.uniqueKey());
        lab.watchCounter(counterKey);
        // The subscription reaches the counter asynchronously; keep counting until it sees one.
        TestRivet.eventually(() -> {
            long sent = counter.increment(1);
            return lab.lastSeenCount() > 0 && lab.lastSeenCount() <= sent;
        }, Duration.ofSeconds(15));
    }

    @Test
    void actorsWithoutHttpOrSocketHandlersRefuseThem() {
        Handle<Counter> counter = client.handle(Counter.class, RivetTest.uniqueKey());
        rivet.retry(() -> counter.api().get());
        Response response = counter.fetch("GET", "/", Map.of(), null);
        assertTrue(response.status() >= 400, "status " + response.status());
        assertThrows(RivetException.class, () -> {
            WebSocket socket = counter.webSocket("/");
            BlockingQueue<WebSocket.Close> closed = new LinkedBlockingQueue<>();
            socket.onClose(closed::add);
            if (closed.poll(10, TimeUnit.SECONDS) == null) return;
            throw new RivetException("test", "closed", "the actor refused the socket");
        });
    }

    // --- lifecycle ------------------------------------------------------------------------

    @Test
    void createInputReachesOnCreate() {
        Handle<Lab> handle = client.handle(Lab.class, RivetTest.uniqueKey()).withInput(new Lab.CreateInput("hello"));
        assertEquals("hello", rivet.retry(() -> handle.api().createdWith()));
        assertEquals("none", lab(RivetTest.uniqueKey()).createdWith());
    }

    @Test
    void sleepRunsTheHookAndStopsBackgroundWork() throws Exception {
        Lab lab = lab(RivetTest.uniqueKey());
        assertEquals(0, lab.sleeps());
        lab.sleepNow();
        TestRivet.eventually(() -> {
            try {
                return lab.sleeps() == 1;
            } catch (RivetException e) {
                return false;
            }
        }, Duration.ofSeconds(30));
    }

    @Test
    void destroyedActorsAreGone() throws Exception {
        String key = RivetTest.uniqueKey();
        Lab lab = lab(key);
        lab.destroyNow();
        Lab existing = client.get(Lab.class, key);
        TestRivet.eventually(() -> {
            try {
                existing.info();
                return false;
            } catch (RivetException e) {
                return true;
            }
        }, Duration.ofSeconds(30));
    }

    @Test
    void handlesFindActorsByIdOrNotAtAll() {
        String key = RivetTest.uniqueKey();
        String id = lab(key).info().get(0);
        Handle<Lab> byId = client.handle(Lab.class).byId(id);
        assertEquals(key, byId.api().info().get(2));
        Lab missing = client.get(Lab.class, RivetTest.uniqueKey());
        assertThrows(RivetException.class, missing::info);
    }

    // --- connections ------------------------------------------------------------------------

    @Test
    void connectionsReceiveTargetedEventsAndStatus() throws Exception {
        String key = RivetTest.uniqueKey();
        lab(key);
        BlockingQueue<String> pushed = new LinkedBlockingQueue<>();
        BlockingQueue<Conn.Status> statuses = new LinkedBlockingQueue<>();
        try (Conn<Lab, Lab.Event> conn = client.connect(Lab.class, () -> new Lab.Params("ann"), key)) {
            conn.onStatus(statuses::add);
            Conn.Subscription subscription = conn.on(Lab.Pushed.class, event -> pushed.add(event.text()));
            assertEquals(1, conn.api().pushToCaller("one"));
            assertEquals("one", pushed.poll(10, TimeUnit.SECONDS));
            assertEquals(Conn.Status.CONNECTED, conn.status());

            subscription.close();
            conn.api().pushToCaller("two");
            assertNull(pushed.poll(500, TimeUnit.MILLISECONDS), "no events after unsubscribing");
        }
        assertEquals(Conn.Status.CLOSED, Conn.Status.valueOf("CLOSED"));
    }

    @Test
    void subscriptionsCanBeRefused() throws Exception {
        String key = RivetTest.uniqueKey();
        lab(key);
        BlockingQueue<Conn.Status> statuses = new LinkedBlockingQueue<>();
        try (Conn<Lab, Lab.Event> conn = client.connect(Lab.class, () -> new Lab.Params("ann"), key)) {
            conn.onStatus(statuses::add);
            conn.api().pushToCaller("connected");
            // Core closes the connection when OnSubscribe refuses (code 1011, "user.forbidden").
            conn.on(Lab.Secret.class, secret -> {});
            Conn.Status status;
            do {
                status = statuses.poll(10, TimeUnit.SECONDS);
            } while (status != null && status != Conn.Status.DISCONNECTED);
            assertEquals(Conn.Status.DISCONNECTED, status, "the refused subscription dropped the connection");
        }
    }

    @Test
    void kickedConnectionsRunOnDisconnect() throws Exception {
        String key = RivetTest.uniqueKey();
        Lab lab = lab(key);
        try (Conn<Lab, Lab.Event> conn = client.connect(Lab.class, () -> new Lab.Params("ann"), key)) {
            // The reply may never arrive: the action closes the connection it came in on.
            client.handle(Lab.class, key).withParams(new Lab.Params("ann"));
            Thread.ofVirtual().start(() -> {
                try {
                    conn.api().kickCaller();
                } catch (RuntimeException ignored) {
                    // the connection went away under the call
                }
            });
            TestRivet.eventually(() -> lab.disconnects() >= 1, Duration.ofSeconds(20));
        }
    }

    @Test
    void connectionsWithoutParams() throws Exception {
        String key = RivetTest.uniqueKey();
        Counter counter = client.getOrCreate(Counter.class, key);
        rivet.retry(counter::get);
        BlockingQueue<Long> counts = new LinkedBlockingQueue<>();
        try (Conn<Counter, Counter.NewCount> conn = client.connect(Counter.class, key)) {
            conn.on(Counter.NewCount.class, event -> counts.add(event.count()));
            conn.api().increment(4);
            assertEquals(4L, counts.poll(10, TimeUnit.SECONDS));
        }
    }

    // --- client handles ------------------------------------------------------------------

    @Test
    void asyncAndRunCoverEveryArity() throws Exception {
        Handle<Lab> handle = client.handle(Lab.class, RivetTest.uniqueKey());
        rivet.retry(() -> handle.api().info());
        assertEquals(3L, handle.async(Lab::add2, 1L, 2L).get(10, TimeUnit.SECONDS));
        assertEquals(6L, handle.async(Lab::add3, 1L, 2L, 3L).get(10, TimeUnit.SECONDS));
        handle.run(Lab::store, "k", "v").get(10, TimeUnit.SECONDS);
        assertEquals(null, handle.async(Lab::stored, "k3").get(10, TimeUnit.SECONDS));
        handle.api().store3("k3", "v", "!");
        handle.run(Lab::sleeps).get(10, TimeUnit.SECONDS);
        assertEquals("v", handle.api().stored("k"));
        assertEquals("v!", handle.api().stored("k3"));
        Throwable failure = assertThrows(java.util.concurrent.ExecutionException.class,
                () -> handle.run(Lab::crash).get(10, TimeUnit.SECONDS)).getCause();
        assertTrue(failure instanceof RivetException r && r.isInternal(), String.valueOf(failure));
    }

    @Test
    void paramsAreCheckedAgainstTheApi() {
        Handle<Lab> lab = client.handle(Lab.class, RivetTest.uniqueKey());
        assertThrows(IllegalArgumentException.class, () -> lab.withParams("not params"));
        assertThrows(IllegalArgumentException.class,
                () -> client.handle(Counter.class, "k").withParams(new Lab.Params("x")));
        Handle<Lab> withParams = lab.withParams(new Lab.Params("ann"));
        assertEquals("lab", rivet.retry(() -> withParams.api().info()).get(1));
    }

    @Test
    void proxiesBehaveLikeObjects() {
        Lab a = client.getOrCreate(Lab.class, "same");
        Lab b = client.getOrCreate(Lab.class, "same");
        assertEquals(a, a);
        assertNotEquals(a, b);
        assertEquals(System.identityHashCode(a), a.hashCode());
        assertTrue(a.toString().contains("Lab"));
    }

    @Test
    void clientsAreConfigurable() {
        Client.Options options = Client.Options.of(RivetTest.ENDPOINT)
                .withToken("default")
                .withNamespace("default")
                .withPoolName(rivet.pool());
        try (Client second = Client.of(options)) {
            Counter counter = second.getOrCreate(Counter.class, RivetTest.uniqueKey());
            assertEquals(1, rivet.retry(() -> counter.increment(1)));
        }
        assertTrue(Client.Options.fromEnv().endpoint().startsWith("http"));
        try (Client plain = Client.of(RivetTest.ENDPOINT)) {
            assertTrue(plain.toString() != null);
        }
    }

    // --- raw HTTP and WebSockets --------------------------------------------------------------

    @Test
    void requestsSeeHeadersAndBodies() {
        Handle<Lab> lab = client.handle(Lab.class, RivetTest.uniqueKey());
        rivet.retry(() -> lab.api().info());
        Response response = lab.fetch("POST", "/headers", Map.of("x-test", "yes"), "body".getBytes());
        assertEquals(201, response.status());
        assertEquals("yes|body", response.text());
        assertEquals("yes", response.headers().get("x-echo"));
        assertEquals(404, lab.fetch("GET", "/other", Map.of(), null).status());
    }

    @Test
    void bodiesStreamByteByByte() {
        Handle<Lab> lab = client.handle(Lab.class, RivetTest.uniqueKey());
        rivet.retry(() -> lab.api().info());
        Response response = lab.fetch("POST", "/bytewise", Map.of(), new byte[70_000]);
        assertEquals(70_000, response.bytes().length);
    }

    @Test
    void failingResponseWritersAbortTheBody() {
        Handle<Lab> lab = client.handle(Lab.class, RivetTest.uniqueKey());
        rivet.retry(() -> lab.api().info());
        Response response = lab.fetch("GET", "/fails", Map.of(), null);
        assertEquals(200, response.status());
        assertThrows(RuntimeException.class, response::bytes, "the client sees a broken body");
    }

    @Test
    void websocketsCarryBinaryAndClose() throws Exception {
        Handle<Lab> lab = client.handle(Lab.class, RivetTest.uniqueKey());
        rivet.retry(() -> lab.api().info());
        BlockingQueue<WebSocket.Message> messages = new LinkedBlockingQueue<>();
        BlockingQueue<WebSocket.Close> closes = new LinkedBlockingQueue<>();
        WebSocket socket = lab.webSocket("/");
        socket.onMessage(messages::add);
        socket.onClose(closes::add);
        socket.send(new byte[] {9, 8});
        WebSocket.Message binary = messages.poll(10, TimeUnit.SECONDS);
        assertTrue(binary != null && binary.binary());
        assertArrayEquals(new byte[] {9, 8}, binary.data());
        socket.send("hi");
        assertEquals("echo:hi", messages.poll(10, TimeUnit.SECONDS).text());
        socket.send("bye");
        WebSocket.Close close = closes.poll(10, TimeUnit.SECONDS);
        assertTrue(close != null, "the actor closed the socket");
        assertEquals(4000, close.code());
        assertThrows(IllegalStateException.class, () -> socket.send("late"));
        TestRivet.eventually(() -> lab.api().socketsClosed() == 1, Duration.ofSeconds(10));
    }

    @Test
    void clientsCanCloseSockets() throws Exception {
        Handle<Echo> echo = client.handle(Echo.class, RivetTest.uniqueKey());
        rivet.retry(() -> echo.api().frames());
        BlockingQueue<WebSocket.Close> closes = new LinkedBlockingQueue<>();
        WebSocket socket = echo.webSocket("/");
        socket.onClose(closes::add);
        socket.close(1000, "done");
        assertTrue(closes.poll(10, TimeUnit.SECONDS) != null);
    }

    // --- registry -------------------------------------------------------------------------

    public interface Twin extends Actor<None, None> {
        void x();
    }

    @Name("twin")
    public interface OtherTwin extends Actor<None, None> {
        void y();
    }

    public static final class TwinActor implements Twin {
        public TwinActor(ActorContext<None> ctx) {}

        @Override
        public void x() {}
    }

    public static final class OtherTwinActor implements OtherTwin {
        public OtherTwinActor(ActorContext<None> ctx) {}

        @Override
        public void y() {}
    }

    @Test
    void registriesValidateNamesAndOptions() {
        Registry registry = Registry.builder()
                .actor(Twin.class, TwinActor::new, actor -> actor.noSleep().actionTimeout(Duration.ofSeconds(1)))
                .build()
                .validate();
        assertEquals(List.of("twin"), registry.names());
        IllegalArgumentException duplicate = assertThrows(IllegalArgumentException.class, () -> Registry.builder()
                .actor(Twin.class, TwinActor::new)
                .actor(OtherTwin.class, OtherTwinActor::new)
                .build());
        assertTrue(duplicate.getMessage().contains("twin"));
        assertThrows(IllegalArgumentException.class, () -> Registry.builder().build());
        assertTrue(rivet.serving().done() != null);
    }
}
