package dev.rivet.kit.internal;

import static java.lang.foreign.ValueLayout.JAVA_BYTE;

import dev.rivet.kit.RivetException;
import dev.rivet.kit.ffi.internal.RivetFfi;
import dev.rivet.kit.ffi.internal.rivet_complete_cb;
import dev.rivet.kit.ffi.internal.rivet_event_cb;
import java.lang.System.Logger.Level;
import java.lang.foreign.Arena;
import java.lang.foreign.MemorySegment;
import java.util.Map;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.atomic.AtomicLong;

/**
 * The only class that calls rivetkit-ffi. See DESIGN.md, "The boundary".
 *
 * <p>Upcalls run on Rust runtime threads. They copy what they need and route the
 * event to a sink, which only enqueues; user code never runs on a Rust thread.
 * Completions finish an internal future inline: waiters are parked threads, and
 * futures handed to users continue on virtual threads ({@link #userFuture}).
 */
public final class Native {
    private static final System.Logger LOG = System.getLogger("dev.rivet.kit");
    private static final System.Logger RUST_LOG = System.getLogger("dev.rivet.kit.native");

    /** Header and body of a completion or reply. */
    public record Answer(byte[] header, byte[] body) {}

    /** Receives the events of one actor type. Runs on a Rust thread: enqueue only. */
    public interface TypeSink {
        void onEvent(int kind, long actor, long id, byte[] header, byte[] body);
    }

    /** Receives events about one resource (stream, socket, client connection). Enqueue only. */
    public interface ResourceSink {
        default void onChunk(MemorySegment data, long lease) {
            leaseRelease(lease);
        }

        default void onEvent(int kind, byte[] header, byte[] body) {}
    }

    private static final AtomicLong NEXT_TOKEN = new AtomicLong(1);
    private record Pending(CompletableFuture<Answer> future, java.util.function.Consumer<Answer> inline) {}

    private static final Map<Long, Pending> PENDING = new ConcurrentHashMap<>();
    private static final Map<Long, TypeSink> TYPES = new ConcurrentHashMap<>();
    private static final Map<Long, ResourceSink> RESOURCES = new ConcurrentHashMap<>();

    static {
        NativeLoader.load();
        int version = RivetFfi.rivet_abi_version();
        if (version != Wire.ABI_VERSION) {
            throw new IllegalStateException("rivetkit native library speaks ABI " + version
                    + " but this SDK speaks " + Wire.ABI_VERSION + "; the jar and the library are from different builds");
        }
        // Upcall stubs live for the whole process: Rust may call them at any time.
        Arena global = Arena.global();
        MemorySegment onEvent = rivet_event_cb.allocate(Native::onEvent, global);
        MemorySegment onComplete = rivet_complete_cb.allocate(Native::onComplete, global);
        byte[] options = Codec.encode(new Wire.InitOptions(System.getProperty("rivet.log", "warn")));
        try (Arena arena = Arena.ofConfined()) {
            if (RivetFfi.rivet_init(onEvent, onComplete, segment(arena, options), options.length) != Wire.STATUS_OK) {
                throw new IllegalStateException("rivet_init failed: the SDK was likely loaded twice in this process "
                        + "(class loader reload is not supported)");
            }
        }
    }

    private Native() {}

    /** Forces library loading and initialization. */
    public static void init() {}

    public static void registerType(long typeId, TypeSink sink) {
        TYPES.put(typeId, sink);
    }

    public static void registerResource(long id, ResourceSink sink) {
        RESOURCES.put(id, sink);
    }

    public static void unregisterResource(long id) {
        RESOURCES.remove(id);
    }

    /** Calls waiting on Rust. Zero when idle; tests use it to check for leaks. */
    public static int pendingCount() {
        return PENDING.size();
    }

    // --- upcalls (Rust runtime threads; never throw, never block) ------------------

    private static void onEvent(
            int kind, long actorType, long actor, long id,
            MemorySegment header, long headerLen, MemorySegment body, long bodyLen, long lease) {
        try {
            if (kind == Wire.STREAM_CHUNK) {
                ResourceSink sink = RESOURCES.get(id);
                MemorySegment view = view(body, bodyLen);
                if (sink != null) sink.onChunk(view, lease);
                else leaseRelease(lease);
                return;
            }
            byte[] head = copy(header, headerLen);
            byte[] data = copy(body, bodyLen);
            if (lease != 0) leaseRelease(lease);
            if (kind == Wire.LOG) {
                log(head);
                return;
            }
            if (actorType != 0 && kind != Wire.STREAM_END && kind != Wire.CLIENT_EVENT && kind != Wire.CLIENT_STATUS) {
                TypeSink sink = TYPES.get(actorType);
                if (sink != null) {
                    sink.onEvent(kind, actor, id, head, data);
                } else if (id != 0 && takesReply(kind)) {
                    replyError(id, new RivetException("actor", "unknown_type", "no actor type " + actorType));
                }
                return;
            }
            long resource = id;
            if (kind == Wire.WS_CLOSE) resource = Codec.decode(head, Wire.WsCloseEvent.class).ws();
            ResourceSink sink = RESOURCES.get(resource);
            if (sink != null) sink.onEvent(kind, head, data);
        } catch (Throwable t) {
            LOG.log(Level.ERROR, "rivetkit on_event upcall failed", t);
        }
    }

    private static boolean takesReply(int kind) {
        return kind < Wire.ABORT || kind == Wire.WS_CLOSE;
    }

    private static void onComplete(
            long token, int status, MemorySegment header, long headerLen, MemorySegment body, long bodyLen, long lease) {
        try {
            byte[] head = copy(header, headerLen);
            byte[] data = copy(body, bodyLen);
            if (lease != 0) leaseRelease(lease);
            Pending pending = PENDING.remove(token);
            if (pending == null) {
                LOG.log(Level.WARNING, "rivetkit completion for unknown token {0}", token);
                return;
            }
            if (status == Wire.STATUS_OK) {
                Answer answer = new Answer(head, data);
                if (pending.inline() != null) pending.inline().accept(answer);
                pending.future().complete(answer);
            } else {
                pending.future().completeExceptionally(error(head));
            }
        } catch (Throwable t) {
            LOG.log(Level.ERROR, "rivetkit on_complete upcall failed", t);
        }
    }

    private static void log(byte[] header) {
        Wire.LogEvent event = Codec.decode(header, Wire.LogEvent.class);
        Level level = switch (event.level()) {
            case "error" -> Level.ERROR;
            case "warn" -> Level.WARNING;
            case "info" -> Level.INFO;
            case "debug" -> Level.DEBUG;
            default -> Level.TRACE;
        };
        RUST_LOG.log(level, "{0}: {1}", event.target(), event.message());
    }

    // Restricted call 1 of 3 in this module (with NativeLoader.load). Rust guarantees
    // len readable bytes for the duration of the upcall, and we copy before returning.
    @SuppressWarnings("restricted")
    private static byte[] copy(MemorySegment buf, long len) {
        if (len == 0) return new byte[0];
        return buf.reinterpret(len).toArray(JAVA_BYTE);
    }

    // A leased buffer stays valid until rivet_lease_release; the reader releases it.
    @SuppressWarnings("restricted")
    private static MemorySegment view(MemorySegment buf, long len) {
        return len == 0 ? MemorySegment.NULL : buf.reinterpret(len);
    }

    // --- downcalls -------------------------------------------------------------------

    /** Runs an op; the future completes (possibly before this returns) with its answer. */
    public static CompletableFuture<Answer> call(long target, int op, Object header, byte[] body) {
        return call(target, op, header, body, null);
    }

    /**
     * Like {@link #call(long, int, Object, byte[])}, running {@code inline} on the
     * completing thread before the future completes. Use it to register a sink for
     * a resource the answer creates, before Rust can send its first event.
     */
    public static CompletableFuture<Answer> call(
            long target, int op, Object header, byte[] body, java.util.function.Consumer<Answer> inline) {
        byte[] head = header == null ? new byte[0] : header instanceof byte[] b ? b : Codec.encode(header);
        long token = NEXT_TOKEN.getAndIncrement();
        CompletableFuture<Answer> future = new CompletableFuture<>();
        PENDING.put(token, new Pending(future, inline));
        int status;
        try (Arena arena = Arena.ofConfined()) {
            status = RivetFfi.rivet_call(
                    target, op, segment(arena, head), head.length, segment(arena, body), body == null ? 0 : body.length, token);
        }
        if (status != Wire.STATUS_OK && RivetFfi.rivet_last_error(token) != Wire.STATUS_OK) {
            if (PENDING.remove(token) != null) {
                future.completeExceptionally(new RivetException("ffi", "call_failed", "rivet_call failed with no error"));
            }
        }
        return future;
    }

    /** Runs an op and waits for it. Interrupts surface as {@code actor/cancelled}. */
    public static Answer await(CompletableFuture<Answer> future) {
        try {
            return future.get();
        } catch (InterruptedException e) {
            Thread.currentThread().interrupt();
            throw new RivetException("actor", "cancelled", "interrupted while waiting", null, e);
        } catch (ExecutionException e) {
            throw RivetException.from(e.getCause());
        }
    }

    public static Answer callSync(long target, int op, Object header, byte[] body) {
        return await(call(target, op, header, body));
    }

    /** A user-facing future: its continuations run on virtual threads, never on Rust threads. */
    public static <T> CompletableFuture<T> userFuture(CompletableFuture<T> internal) {
        return internal.whenCompleteAsync((value, error) -> {}, Threads.VIRTUAL);
    }

    public static void reply(long event, byte[] header, byte[] body) {
        reply(event, Wire.STATUS_OK, header, body);
    }

    public static void replyError(long event, Throwable error) {
        RivetException rivet = RivetException.from(error);
        if (rivet.isInternal()) {
            LOG.log(Level.ERROR, "rivetkit actor code failed", rivet.getCause() != null ? rivet.getCause() : rivet);
        }
        reply(event, Wire.STATUS_ERROR, Codec.encode(body(rivet)), null);
    }

    static Wire.ErrorBody body(RivetException error) {
        return new Wire.ErrorBody(error.group(), error.code(), error.detail(), error.metadata());
    }

    private static void reply(long event, int status, byte[] header, byte[] body) {
        try (Arena arena = Arena.ofConfined()) {
            int result = RivetFfi.rivet_reply(
                    event, status, segment(arena, header), header == null ? 0 : header.length,
                    segment(arena, body), body == null ? 0 : body.length);
            if (result != Wire.STATUS_OK) {
                // Rust already gave up on this event (timeout, cancel, or shutdown).
                LOG.log(Level.DEBUG, "rivet_reply rejected event {0}", event);
            }
        }
    }

    public static void leaseRelease(long lease) {
        if (lease != 0) RivetFfi.rivet_lease_release(lease);
    }

    /** Releases a handle. Returns false when Rust no longer knew it. */
    public static boolean release(long handle) {
        return RivetFfi.rivet_release(handle) == Wire.STATUS_OK;
    }

    static RivetException error(byte[] header) {
        try {
            Wire.ErrorBody e = Codec.decode(header, Wire.ErrorBody.class);
            return new RivetException(e.group(), e.code(), e.message(), e.metadata(), null);
        } catch (RuntimeException decodeError) {
            return new RivetException("ffi", "bad_error_body", "undecodable error body", null, decodeError);
        }
    }

    private static MemorySegment segment(Arena arena, byte[] bytes) {
        return bytes == null || bytes.length == 0 ? MemorySegment.NULL : arena.allocateFrom(JAVA_BYTE, bytes);
    }

    public static long handleOf(Answer answer) {
        return Codec.decode(answer.header(), Wire.Created.class).handle();
    }
}
