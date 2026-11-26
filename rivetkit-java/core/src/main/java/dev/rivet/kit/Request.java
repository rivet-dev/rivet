package dev.rivet.kit;

import java.io.IOException;
import java.io.InputStream;
import java.io.UncheckedIOException;
import java.util.Map;
import java.util.Optional;

/**
 * An HTTP request. Header names are lowercase. A large or streamed body is read from
 * {@link #body()} as it arrives; the sender is paused while you do not read.
 */
public record Request(String method, String uri, Map<String, String> headers, InputStream body) {
    public Optional<String> header(String name) {
        return Optional.ofNullable(headers.get(name.toLowerCase(java.util.Locale.ROOT)));
    }

    /** Reads the whole body. */
    public byte[] bytes() {
        try (InputStream in = body) {
            return in.readAllBytes();
        } catch (IOException e) {
            throw new UncheckedIOException(e);
        }
    }

    public String text() {
        return new String(bytes(), java.nio.charset.StandardCharsets.UTF_8);
    }

    /** The path part of {@link #uri()}. */
    public String path() {
        int q = uri.indexOf('?');
        return q < 0 ? uri : uri.substring(0, q);
    }
}
