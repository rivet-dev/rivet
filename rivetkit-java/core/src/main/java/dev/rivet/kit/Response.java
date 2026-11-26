package dev.rivet.kit;

import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.io.UncheckedIOException;
import java.nio.charset.StandardCharsets;
import java.util.Map;

/**
 * An HTTP response: a whole body, or a {@link Writer} that streams it. A streaming
 * writer runs after the status and headers are sent, as a background turn; each
 * write waits until the client has room (backpressure).
 */
public record Response(int status, Map<String, String> headers, byte[] body, Writer writer, InputStream stream) {
    @FunctionalInterface
    public interface Writer {
        void write(OutputStream out) throws Exception;
    }

    public static Response of(int status, byte[] body, Map<String, String> headers) {
        return new Response(status, headers, body, null, null);
    }

    public static Response text(int status, String body) {
        return of(status, body.getBytes(StandardCharsets.UTF_8), Map.of("content-type", "text/plain; charset=utf-8"));
    }

    public static Response streaming(int status, Map<String, String> headers, Writer writer) {
        return new Response(status, headers, null, writer, null);
    }

    /** A client-side response whose body is read as it arrives. */
    public static Response received(int status, Map<String, String> headers, InputStream body) {
        return new Response(status, headers, null, null, body);
    }

    /** The whole body (reads a received stream to the end). */
    public byte[] bytes() {
        if (body != null) return body;
        if (stream == null) return new byte[0];
        try (InputStream in = stream) {
            return in.readAllBytes();
        } catch (IOException e) {
            throw new UncheckedIOException(e);
        }
    }

    public String text() {
        return new String(bytes(), StandardCharsets.UTF_8);
    }
}
