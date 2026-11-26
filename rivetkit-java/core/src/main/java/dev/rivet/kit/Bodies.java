package dev.rivet.kit;

import java.io.IOException;
import java.io.InputStream;
import java.io.UncheckedIOException;
import java.nio.charset.StandardCharsets;

/** Whole-body reads shared by {@link Request} and {@link FetchResponse}. */
final class Bodies {
    private Bodies() {}

    /** Reads {@code body} to the end and closes it. */
    static byte[] readAll(InputStream body) {
        try (InputStream in = body) {
            return in.readAllBytes();
        } catch (IOException e) {
            throw new UncheckedIOException(e);
        }
    }

    static String text(byte[] bytes) {
        return new String(bytes, StandardCharsets.UTF_8);
    }

    static void close(InputStream body) {
        try {
            body.close();
        } catch (IOException e) {
            throw new UncheckedIOException(e);
        }
    }
}
