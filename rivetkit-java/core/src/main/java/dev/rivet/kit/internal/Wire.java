package dev.rivet.kit.internal;

import com.fasterxml.jackson.annotation.JsonInclude;
import dev.rivet.kit.ffi.internal.RivetFfi;
import java.util.List;
import java.util.Map;

/**
 * The wire protocol's constants and CBOR headers, mirroring
 * {@code rivetkit-host-adapter/src/wire/messages.rs}. Kinds and ops come from the
 * generated header, so a renumbering on the Rust side breaks the build here.
 * {@code FixturesTest} checks every header against Rust's golden samples.
 */
public final class Wire {
    private Wire() {}

    /** The ABI version this Java code speaks. */
    public static final int ABI_VERSION = 3;

    // Event kinds.
    public static final int MIGRATE = RivetFfi.RIVET_EVENT_KIND_MIGRATE();
    public static final int START = RivetFfi.RIVET_EVENT_KIND_START();
    public static final int CREATE = RivetFfi.RIVET_EVENT_KIND_CREATE();
    public static final int FORK = RivetFfi.RIVET_EVENT_KIND_FORK();
    public static final int WAKE = RivetFfi.RIVET_EVENT_KIND_WAKE();
    public static final int ACTION = RivetFfi.RIVET_EVENT_KIND_ACTION();
    public static final int BEFORE_CONNECT = RivetFfi.RIVET_EVENT_KIND_BEFORE_CONNECT();
    public static final int CONNECT = RivetFfi.RIVET_EVENT_KIND_CONNECT();
    public static final int DISCONNECT = RivetFfi.RIVET_EVENT_KIND_DISCONNECT();
    public static final int SUBSCRIBE = RivetFfi.RIVET_EVENT_KIND_SUBSCRIBE();
    public static final int REQUEST = RivetFfi.RIVET_EVENT_KIND_REQUEST();
    public static final int WEB_SOCKET = RivetFfi.RIVET_EVENT_KIND_WEB_SOCKET();
    public static final int SLEEP = RivetFfi.RIVET_EVENT_KIND_SLEEP();
    public static final int DESTROY = RivetFfi.RIVET_EVENT_KIND_DESTROY();
    public static final int ABORT = RivetFfi.RIVET_EVENT_KIND_ABORT();
    public static final int RELEASED = RivetFfi.RIVET_EVENT_KIND_RELEASED();
    public static final int CANCEL = RivetFfi.RIVET_EVENT_KIND_CANCEL();
    public static final int STREAM_CHUNK = RivetFfi.RIVET_EVENT_KIND_STREAM_CHUNK();
    public static final int STREAM_END = RivetFfi.RIVET_EVENT_KIND_STREAM_END();
    public static final int WS_MESSAGE = RivetFfi.RIVET_EVENT_KIND_WS_MESSAGE();
    public static final int WS_CLOSE = RivetFfi.RIVET_EVENT_KIND_WS_CLOSE();
    public static final int CLIENT_EVENT = RivetFfi.RIVET_EVENT_KIND_CLIENT_EVENT();
    public static final int CLIENT_STATUS = RivetFfi.RIVET_EVENT_KIND_CLIENT_STATUS();
    public static final int CONN_PARAMS = RivetFfi.RIVET_EVENT_KIND_CONN_PARAMS();
    public static final int LOG = RivetFfi.RIVET_EVENT_KIND_LOG();

    // Ops.
    public static final int OP_REGISTRY_NEW = RivetFfi.RIVET_OP_REGISTRY_NEW();
    public static final int OP_CLIENT_NEW = RivetFfi.RIVET_OP_CLIENT_NEW();
    public static final int OP_REGISTER = RivetFfi.RIVET_OP_REGISTER();
    public static final int OP_SERVE = RivetFfi.RIVET_OP_SERVE();
    public static final int OP_SERVE_DONE = RivetFfi.RIVET_OP_SERVE_DONE();
    public static final int OP_STOP = RivetFfi.RIVET_OP_STOP();
    public static final int OP_BROADCAST = RivetFfi.RIVET_OP_BROADCAST();
    public static final int OP_CONN_SEND = RivetFfi.RIVET_OP_CONN_SEND();
    public static final int OP_CONN_DISCONNECT = RivetFfi.RIVET_OP_CONN_DISCONNECT();
    public static final int OP_CONN_LIST = RivetFfi.RIVET_OP_CONN_LIST();
    public static final int OP_SLEEP = RivetFfi.RIVET_OP_SLEEP();
    public static final int OP_DESTROY = RivetFfi.RIVET_OP_DESTROY();
    public static final int OP_KEEP_AWAKE = RivetFfi.RIVET_OP_KEEP_AWAKE();
    public static final int OP_WAIT_UNTIL = RivetFfi.RIVET_OP_WAIT_UNTIL();
    public static final int OP_ACTOR_CLIENT = RivetFfi.RIVET_OP_ACTOR_CLIENT();
    public static final int OP_SQL_EXEC = RivetFfi.RIVET_OP_SQL_EXEC();
    public static final int OP_SQL_QUERY = RivetFfi.RIVET_OP_SQL_QUERY();
    public static final int OP_SQL_RUN = RivetFfi.RIVET_OP_SQL_RUN();
    public static final int OP_SQL_EXECUTE = RivetFfi.RIVET_OP_SQL_EXECUTE();
    public static final int OP_SQL_BATCH = RivetFfi.RIVET_OP_SQL_BATCH();
    public static final int OP_SQL_BEGIN = RivetFfi.RIVET_OP_SQL_BEGIN();
    public static final int OP_SQL_COMMIT = RivetFfi.RIVET_OP_SQL_COMMIT();
    public static final int OP_SQL_ROLLBACK = RivetFfi.RIVET_OP_SQL_ROLLBACK();
    public static final int OP_SCHEDULE = RivetFfi.RIVET_OP_SCHEDULE();
    public static final int OP_SCHEDULE_CANCEL = RivetFfi.RIVET_OP_SCHEDULE_CANCEL();
    public static final int OP_SCHEDULE_LIST = RivetFfi.RIVET_OP_SCHEDULE_LIST();
    public static final int OP_CRON_SET = RivetFfi.RIVET_OP_CRON_SET();
    public static final int OP_CRON_DELETE = RivetFfi.RIVET_OP_CRON_DELETE();
    public static final int OP_CRON_LIST = RivetFfi.RIVET_OP_CRON_LIST();
    public static final int OP_STREAM_WRITE = RivetFfi.RIVET_OP_STREAM_WRITE();
    public static final int OP_STREAM_ERROR = RivetFfi.RIVET_OP_STREAM_ERROR();
    public static final int OP_WS_SEND = RivetFfi.RIVET_OP_WS_SEND();
    public static final int OP_WS_CLOSE = RivetFfi.RIVET_OP_WS_CLOSE();
    public static final int OP_CLIENT_ACTION = RivetFfi.RIVET_OP_CLIENT_ACTION();
    public static final int OP_CLIENT_CONNECT = RivetFfi.RIVET_OP_CLIENT_CONNECT();
    public static final int OP_CLIENT_FETCH = RivetFfi.RIVET_OP_CLIENT_FETCH();
    public static final int OP_CLIENT_WEB_SOCKET = RivetFfi.RIVET_OP_CLIENT_WEB_SOCKET();
    public static final int OP_CONN_ACTION = RivetFfi.RIVET_OP_CONN_ACTION();
    public static final int OP_CONN_SUBSCRIBE = RivetFfi.RIVET_OP_CONN_SUBSCRIBE();
    public static final int OP_CONN_UNSUBSCRIBE = RivetFfi.RIVET_OP_CONN_UNSUBSCRIBE();

    public static final int STATUS_OK = RivetFfi.RIVET_STATUS_OK();
    public static final int STATUS_ERROR = RivetFfi.RIVET_STATUS_ERROR();

    // --- headers ------------------------------------------------------------------
    // Records map to CBOR maps with these exact (camelCase) keys. Null fields are
    // omitted, matching Rust's skip_serializing_if.

    @JsonInclude(JsonInclude.Include.NON_NULL)
    public record ErrorBody(String group, String code, String message, Object metadata) {}

    public record Empty() {}

    public record Created(long handle) {}

    public record ActorInfo(String actorId, String name, List<String> key, String region) {}

    public record MigrateEvent(boolean isNew) {}

    public record StartEvent(ActorInfo info) {}

    public record CreateEvent(boolean hasInput) {}

    public record ForkEvent(String parentActorId, List<String> parentKey, long forkedAt, boolean hasInput) {}

    @JsonInclude(JsonInclude.Include.NON_NULL)
    public record ActionEvent(String name, String connId, boolean scheduled) {}

    public record RequestHead(String method, String uri, Map<String, String> headers) {}

    @JsonInclude(JsonInclude.Include.NON_NULL)
    public record ConnEvent(String connId, RequestHead request) {}

    public record SubscribeEvent(String connId, String event) {}

    @JsonInclude(JsonInclude.Include.NON_NULL)
    public record RequestEvent(RequestHead request, Long bodyStream, long responseBody) {}

    @JsonInclude(JsonInclude.Include.NON_NULL)
    public record RequestReply(Integer status, Map<String, String> headers, boolean streamed) {}

    @JsonInclude(JsonInclude.Include.NON_NULL)
    public record WebSocketEvent(String connId, long ws, RequestHead request) {}

    @JsonInclude(JsonInclude.Include.NON_NULL)
    public record WsMessageEvent(boolean binary, Integer index) {}

    public record WsCloseEvent(long ws, int code, String reason, boolean wasClean) {}

    @JsonInclude(JsonInclude.Include.NON_NULL)
    public record StreamEndEvent(ErrorBody error) {}

    public record CancelEvent(long event) {}

    public record ClientEventEvent(String name) {}

    @JsonInclude(JsonInclude.Include.NON_NULL)
    public record ClientStatusEvent(String status, ErrorBody error) {}

    public record ConnParamsEvent(long source) {}

    public record LogEvent(String level, String target, String message) {}

    @JsonInclude(JsonInclude.Include.NON_NULL)
    public record InitOptions(String logFilter) {}

    /** Which hooks the actor class implements. Field names match Rust's {@code Hooks}. */
    public record Hooks(
            boolean onMigrate,
            boolean onCreate,
            boolean onFork,
            boolean onWake,
            boolean onSleep,
            boolean onDestroy,
            boolean onBeforeConnect,
            boolean onConnect,
            boolean onDisconnect,
            boolean onSubscribe,
            boolean onRequest,
            boolean onWebsocket,
            boolean onAbort) {}

    @JsonInclude(JsonInclude.Include.NON_NULL)
    public record RegisterOptions(
            String name,
            long actorType,
            Hooks hooks,
            Integer actionTimeoutMs,
            Integer sleepTimeoutMs,
            Boolean noSleep,
            Integer hookTimeoutMs,
            Integer requestTimeoutMs) {}

    @JsonInclude(JsonInclude.Include.NON_NULL)
    public record ServeOptions(String endpoint, String token, String namespace, String poolName, String engineSpawn) {}

    public record BroadcastCall(String name) {}

    public record ConnSendCall(String connId, String name) {}

    @JsonInclude(JsonInclude.Include.NON_NULL)
    public record ConnDisconnectCall(String connId, String reason) {}

    public record ConnInfo(String id, byte[] params) {}

    public record ConnList(List<ConnInfo> conns) {}

    @JsonInclude(JsonInclude.Include.NON_NULL)
    public record SqlCall(String sql, List<Object> params) {}

    public record SqlBatchCall(List<SqlCall> statements) {}

    @JsonInclude(JsonInclude.Include.NON_NULL)
    public record SqlResult(List<String> columns, long changes, Long lastInsertRowId) {}

    public record SqlBatchResult(List<SqlResult> results) {}

    @JsonInclude(JsonInclude.Include.NON_NULL)
    public record SqlBeginCall(String name, Integer timeoutMs) {}

    @JsonInclude(JsonInclude.Include.NON_NULL)
    public record ScheduleCall(String action, Long delayMs, Long timestampMs) {}

    public record ScheduleId(String id) {}

    public record Removed(boolean removed) {}

    public record ScheduledEvent(String id, String action, byte[] args, long runAt) {}

    public record ScheduleList(List<ScheduledEvent> events) {}

    @JsonInclude(JsonInclude.Include.NON_NULL)
    public record CronSetCall(
            String name, String action, Long intervalMs, String expression, String timezone, Long maxHistory) {}

    public record NameCall(String name) {}

    @JsonInclude(JsonInclude.Include.NON_NULL)
    public record CronJob(
            String name,
            String action,
            byte[] args,
            long nextRunAt,
            Long lastRunAt,
            String expression,
            String timezone,
            Long intervalMs) {}

    public record CronList(List<CronJob> jobs) {}

    public record StreamWriteCall(boolean fin) {}

    public record StreamErrorCall(String message) {}

    public record WsSendCall(boolean binary) {}

    @JsonInclude(JsonInclude.Include.NON_NULL)
    public record WsCloseCall(Integer code, String reason) {}

    @JsonInclude(JsonInclude.Include.NON_NULL)
    public record ClientOptions(String endpoint, String token, String namespace, String poolName) {}

    @JsonInclude(JsonInclude.Include.NON_NULL)
    public record ActorTarget(
            String actor, List<String> key, String actorId, boolean noCreate, byte[] params, byte[] input) {}

    public record ClientActionCall(ActorTarget target, String action) {}

    /** {@code paramsSource}: when set, Rust asks for params with CONN_PARAMS before every dial. */
    public record ClientConnectCall(ActorTarget target, Long paramsSource) {}

    public record ConnActionCall(String action) {}

    public record ConnSubscribeCall(String event) {}

    public record ClientFetchCall(ActorTarget target, String method, String path, Map<String, String> headers) {}

    public record FetchResult(int status, Map<String, String> headers, long body) {}

    public record ClientWebSocketCall(ActorTarget target, String path, List<String> protocols) {}
}
