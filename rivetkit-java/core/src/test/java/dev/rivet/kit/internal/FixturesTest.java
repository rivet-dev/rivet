package dev.rivet.kit.internal;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.TreeSet;
import java.util.stream.Collectors;
import org.junit.jupiter.api.Test;

/**
 * Drift check against the golden CBOR headers in rivetkit-host-adapter/fixtures
 * (written by messages.rs). For each fixture: Java decodes it into its Wire record,
 * encodes that record again, and the result must be the same CBOR value. A renamed,
 * missing or retyped field on either side fails here. Every fixture must be mapped.
 */
class FixturesTest {
    /** Fixture name to the Java type that reads it. */
    private static final Map<String, Class<?>> TYPES = Map.ofEntries(
            Map.entry("event_migrate", Wire.MigrateEvent.class),
            Map.entry("event_start", Wire.StartEvent.class),
            Map.entry("event_create", Wire.CreateEvent.class),
            Map.entry("event_fork", Wire.ForkEvent.class),
            Map.entry("event_action", Wire.ActionEvent.class),
            Map.entry("event_conn", Wire.ConnEvent.class),
            Map.entry("event_subscribe", Wire.SubscribeEvent.class),
            Map.entry("event_request", Wire.RequestEvent.class),
            Map.entry("event_websocket", Wire.WebSocketEvent.class),
            Map.entry("event_cancel", Wire.CancelEvent.class),
            Map.entry("event_stream_end", Wire.StreamEndEvent.class),
            Map.entry("event_ws_message", Wire.WsMessageEvent.class),
            Map.entry("event_ws_close", Wire.WsCloseEvent.class),
            Map.entry("event_client_event", Wire.ClientEventEvent.class),
            Map.entry("event_client_status", Wire.ClientStatusEvent.class),
            Map.entry("event_log", Wire.LogEvent.class),
            Map.entry("call_client_new", Wire.ClientOptions.class),
            Map.entry("call_register", Wire.RegisterOptions.class),
            Map.entry("call_serve", Wire.ServeOptions.class),
            Map.entry("call_broadcast", Wire.BroadcastCall.class),
            Map.entry("call_conn_send", Wire.ConnSendCall.class),
            Map.entry("call_conn_disconnect", Wire.ConnDisconnectCall.class),
            Map.entry("call_sql", Wire.SqlCall.class),
            Map.entry("call_sql_batch", Wire.SqlBatchCall.class),
            Map.entry("call_sql_begin", Wire.SqlBeginCall.class),
            Map.entry("call_schedule", Wire.ScheduleCall.class),
            Map.entry("call_schedule_id", Wire.ScheduleId.class),
            Map.entry("call_cron_set", Wire.CronSetCall.class),
            Map.entry("call_name", Wire.NameCall.class),
            Map.entry("call_stream_write", Wire.StreamWriteCall.class),
            Map.entry("call_stream_error", Wire.StreamErrorCall.class),
            Map.entry("call_ws_send", Wire.WsSendCall.class),
            Map.entry("call_ws_close", Wire.WsCloseCall.class),
            Map.entry("call_client_action", Wire.ClientActionCall.class),
            Map.entry("call_client_connect", Wire.ClientConnectCall.class),
            Map.entry("call_client_fetch", Wire.ClientFetchCall.class),
            Map.entry("call_client_websocket", Wire.ClientWebSocketCall.class),
            Map.entry("call_conn_action", Wire.ConnActionCall.class),
            Map.entry("call_conn_subscribe", Wire.ConnSubscribeCall.class),
            Map.entry("reply_request", Wire.RequestReply.class),
            Map.entry("result_created", Wire.Created.class),
            Map.entry("result_sql", Wire.SqlResult.class),
            Map.entry("result_conn_list", Wire.ConnList.class),
            Map.entry("result_schedule_list", Wire.ScheduleList.class),
            Map.entry("result_cron_list", Wire.CronList.class),
            Map.entry("result_fetch", Wire.FetchResult.class),
            Map.entry("result_removed", Wire.Removed.class),
            Map.entry("error_body", Wire.ErrorBody.class),
            Map.entry("init_options", Wire.InitOptions.class));

    /** Fixtures that are not a header record (checked separately). */
    private static final Set<String> OTHER = Set.of("result_sql_rows");

    private static Path dir() {
        return Path.of(System.getProperty("rivet.fixtures"));
    }

    private static Set<String> fixtureNames() throws IOException {
        try (var files = Files.list(dir())) {
            return files.map(p -> p.getFileName().toString())
                    .filter(n -> n.endsWith(".cbor"))
                    .map(n -> n.substring(0, n.length() - 5))
                    .collect(Collectors.toCollection(TreeSet::new));
        }
    }

    @Test
    void everyFixtureIsMapped() throws IOException {
        Set<String> unmapped = new TreeSet<>(fixtureNames());
        unmapped.removeAll(TYPES.keySet());
        unmapped.removeAll(OTHER);
        assertTrue(unmapped.isEmpty(), "fixtures without a Java type: " + unmapped);
        Set<String> missing = new TreeSet<>(TYPES.keySet());
        missing.removeAll(fixtureNames());
        assertTrue(missing.isEmpty(), "Java types without a fixture: " + missing);
    }

    @Test
    void javaReadsAndWritesEveryHeaderLikeRust() throws IOException {
        for (var entry : TYPES.entrySet()) {
            byte[] rust = Files.readAllBytes(dir().resolve(entry.getKey() + ".cbor"));
            Object decoded = Codec.decode(rust, entry.getValue());
            byte[] java = Codec.encode(decoded);
            assertEquals(plain(rust), plain(java), entry.getKey() + ": Java's encoding differs from Rust's");
        }
    }

    @Test
    void sqlRowsKeepTheirTypes() throws IOException {
        byte[] rust = Files.readAllBytes(dir().resolve("result_sql_rows.cbor"));
        List<?> row = (List<?>) ((List<?>) Codec.MAPPER.readValue(rust, List.class)).get(0);
        assertEquals(4, ((Number) row.get(0)).intValue());
        assertEquals("hi", row.get(1));
        assertTrue(row.get(2) instanceof byte[], "blobs arrive as byte[]");
        assertEquals(null, row.get(3));
        assertEquals(0.25, ((Number) row.get(4)).doubleValue());
    }

    /** CBOR as plain values, with byte strings made comparable. */
    private static Object plain(byte[] bytes) {
        return normalize(Codec.MAPPER.readValue(bytes, Object.class));
    }

    private static Object normalize(Object value) {
        if (value instanceof byte[] b) return "bytes:" + java.util.HexFormat.of().formatHex(b);
        if (value instanceof Map<?, ?> m) {
            Map<String, Object> out = new java.util.HashMap<>();
            m.forEach((k, v) -> { if (v != null) out.put(String.valueOf(k), normalize(v)); });
            return out;
        }
        if (value instanceof List<?> l) return l.stream().map(FixturesTest::normalize).toList();
        if (value instanceof Number n && !(value instanceof Double || value instanceof Float)) return n.longValue();
        return value;
    }
}
