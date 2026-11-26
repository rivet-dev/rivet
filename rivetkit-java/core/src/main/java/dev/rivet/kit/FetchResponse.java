package dev.rivet.kit;

import java.io.InputStream;
import java.util.Map;

/**
 * The client side of {@link Handle#fetch}: status and headers, and a body read as it
 * arrives. Read it with {@link #bytes}/{@link #text}, or stream {@link #body}; close the
 * response if you do not read the body to the end, so its buffers are released.
 */
public final class FetchResponse implements AutoCloseable {
    private final int status;
    private final Map<String, String> headers;
    private final InputStream body;

    /** Built by the SDK; tests can build one to stand in for a fetch. */
    public FetchResponse(int status, Map<String, String> headers, InputStream body) {
        this.status = status;
        this.headers = headers == null ? Map.of() : Map.copyOf(headers);
        this.body = body == null ? InputStream.nullInputStream() : body;
    }

    public int status() {
        return status;
    }

    public Map<String, String> headers() {
        return headers;
    }

    /** The body as it arrives. */
    public InputStream body() {
        return body;
    }

    /** The whole body; reads the stream to the end and closes it, so call it once. */
    public byte[] bytes() {
        return Bodies.readAll(body);
    }

    public String text() {
        return Bodies.text(bytes());
    }

    @Override
    public void close() {
        Bodies.close(body);
    }

    @Override
    public String toString() {
        return "FetchResponse[" + status + "]";
    }
}
