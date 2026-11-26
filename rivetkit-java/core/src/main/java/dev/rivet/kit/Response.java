package dev.rivet.kit;

import java.io.OutputStream;
import java.nio.charset.StandardCharsets;
import java.util.Map;

/**
 * What {@link OnRequest#onRequest} returns: a whole body ({@link Bytes}) or a {@link Writer}
 * that streams it ({@link Streaming}). A streaming writer runs after the status and headers are
 * sent, as a background turn; each write waits until the client has room (backpressure).
 * The client side of a fetch is {@link FetchResponse}.
 */
public sealed interface Response permits Response.Bytes, Response.Streaming {
    int status();

    Map<String, String> headers();

    /** A response whose whole body is known up front. */
    record Bytes(int status, Map<String, String> headers, byte[] body) implements Response {
        public Bytes {
            headers = headers == null ? Map.of() : Map.copyOf(headers);
            body = body == null ? new byte[0] : body;
        }
    }

    /** A response whose body {@code writer} streams after the head is sent. */
    record Streaming(int status, Map<String, String> headers, Writer writer) implements Response {
        public Streaming {
            headers = headers == null ? Map.of() : Map.copyOf(headers);
            java.util.Objects.requireNonNull(writer, "writer");
        }
    }

    @FunctionalInterface
    interface Writer {
        void write(OutputStream out) throws Exception;
    }

    static Response of(int status, byte[] body, Map<String, String> headers) {
        return new Bytes(status, headers, body);
    }

    static Response text(int status, String body) {
        return of(status, body.getBytes(StandardCharsets.UTF_8), Map.of("content-type", "text/plain; charset=utf-8"));
    }

    static Response streaming(int status, Map<String, String> headers, Writer writer) {
        return new Streaming(status, headers, writer);
    }
}
