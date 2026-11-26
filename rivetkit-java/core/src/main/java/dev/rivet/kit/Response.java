package dev.rivet.kit;

import java.io.OutputStream;
import java.nio.charset.StandardCharsets;
import java.util.Map;
import java.util.Objects;

/**
 * What {@link OnRequest#onRequest} returns: a whole body ({@link Bytes}) or a {@link Writer}
 * that streams it ({@link Streaming}). A streaming writer runs after the status and headers are
 * sent, as a background turn; each write waits until the client has room (backpressure).
 * The client side of a fetch is {@link FetchResponse}.
 */
public sealed interface Response permits Response.Bytes, Response.Streaming {
    int status();

    Map<String, String> headers();

    @FunctionalInterface
    interface Writer {
        void write(OutputStream out) throws Exception;
    }

    static Response of(int status, Map<String, String> headers, byte[] body) {
        return new Bytes(status, headers, body);
    }

    static Response text(int status, String body) {
        return of(status, Map.of("content-type", "text/plain; charset=utf-8"), body.getBytes(StandardCharsets.UTF_8));
    }

    static Response streaming(int status, Map<String, String> headers, Writer writer) {
        return new Streaming(status, headers, writer);
    }

    /** A response whose whole body is known up front. */
    final class Bytes implements Response {
        private final int status;
        private final Map<String, String> headers;
        private final byte[] body;

        private Bytes(int status, Map<String, String> headers, byte[] body) {
            this.status = status;
            this.headers = headers == null ? Map.of() : Map.copyOf(headers);
            this.body = body == null ? new byte[0] : body;
        }

        @Override
        public int status() {
            return status;
        }

        @Override
        public Map<String, String> headers() {
            return headers;
        }

        public byte[] body() {
            return body;
        }

        @Override
        public String toString() {
            return "Response.Bytes[" + status + ", " + body.length + " bytes]";
        }
    }

    /** A response whose body the {@link #writer()} streams after the head is sent. */
    final class Streaming implements Response {
        private final int status;
        private final Map<String, String> headers;
        private final Writer writer;

        private Streaming(int status, Map<String, String> headers, Writer writer) {
            this.status = status;
            this.headers = headers == null ? Map.of() : Map.copyOf(headers);
            this.writer = Objects.requireNonNull(writer, "writer");
        }

        @Override
        public int status() {
            return status;
        }

        @Override
        public Map<String, String> headers() {
            return headers;
        }

        public Writer writer() {
            return writer;
        }

        @Override
        public String toString() {
            return "Response.Streaming[" + status + "]";
        }
    }
}
