package dev.rivet.kit;

import java.io.InputStream;
import java.util.Locale;
import java.util.Map;
import java.util.Objects;
import java.util.Optional;

/**
 * An HTTP request. Header names are lowercase. A large or streamed body is read from
 * {@link #body()} as it arrives; the sender is paused while you do not read. The SDK closes
 * the body when the response is done, read or not.
 */
public final class Request {
    private final String method;
    private final String uri;
    private final Map<String, String> headers;
    private final InputStream body;

    /** {@code headers} names should be lowercase; tests can build requests with this. */
    public Request(String method, String uri, Map<String, String> headers, InputStream body) {
        this.method = Objects.requireNonNull(method, "method");
        this.uri = Objects.requireNonNull(uri, "uri");
        this.headers = headers == null ? Map.of() : Map.copyOf(headers);
        this.body = body == null ? InputStream.nullInputStream() : body;
    }

    public String method() {
        return method;
    }

    public String uri() {
        return uri;
    }

    public Map<String, String> headers() {
        return headers;
    }

    public InputStream body() {
        return body;
    }

    public Optional<String> header(String name) {
        return Optional.ofNullable(headers.get(name.toLowerCase(Locale.ROOT)));
    }

    /** Reads the whole body. */
    public byte[] bytes() {
        return Bodies.readAll(body);
    }

    public String text() {
        return Bodies.text(bytes());
    }

    /** The path part of {@link #uri()}. */
    public String path() {
        int q = uri.indexOf('?');
        return q < 0 ? uri : uri.substring(0, q);
    }

    @Override
    public String toString() {
        return "Request[" + method + " " + uri + "]";
    }
}
