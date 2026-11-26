package dev.rivet.kit;

import java.io.IOException;
import java.io.InputStream;
import java.io.UncheckedIOException;
import java.nio.charset.StandardCharsets;
import java.util.Map;

/**
 * The client side of {@link Handle#fetch}: status and headers, and a body read as it
 * arrives. Read it with {@link #bytes}/{@link #text}, or stream {@link #body}; close the
 * response if you do not read the body to the end, so its buffers are released.
 */
public record FetchResponse(int status, Map<String, String> headers, InputStream body) implements AutoCloseable {
    /** The whole body; reads the stream to the end and closes it. */
    public byte[] bytes() {
        try (InputStream in = body) {
            return in.readAllBytes();
        } catch (IOException e) {
            throw new UncheckedIOException(e);
        }
    }

    public String text() {
        return new String(bytes(), StandardCharsets.UTF_8);
    }

    @Override
    public void close() {
        try {
            body.close();
        } catch (IOException e) {
            throw new UncheckedIOException(e);
        }
    }
}
