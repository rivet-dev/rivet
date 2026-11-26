package dev.rivet.kit.internal;

import java.io.IOException;
import java.io.InputStream;
import java.io.UncheckedIOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.util.Locale;

/**
 * Loads the rivetkit-ffi native library.
 *
 * <p>Order: the {@code rivet.ffi.library} system property (an absolute path), then the
 * copy packaged at {@code native/<os>-<arch>/} on the classpath, extracted to a temp
 * directory ({@code rivet.ffi.tmpdir}, default {@code java.io.tmpdir}; set it when
 * /tmp is mounted noexec). After {@link System#load}, jextract's bindings find the symbols through
 * {@code SymbolLookup.loaderLookup()}.
 */
final class NativeLoader {
    private NativeLoader() {}

    // Restricted calls 2 and 3 of 3: System.load of our own library.
    @SuppressWarnings("restricted")
    static void load() {
        String override = System.getProperty("rivet.ffi.library");
        if (override != null) {
            System.load(Path.of(override).toAbsolutePath().toString());
            return;
        }
        String resource = "/native/" + platform() + "/" + System.mapLibraryName("rivetkit_ffi");
        try (InputStream in = NativeLoader.class.getResourceAsStream(resource)) {
            if (in == null) {
                throw new UnsatisfiedLinkError(
                        "rivetkit-ffi is not packaged for " + platform() + " (looked for " + resource + ")");
            }
            String base = System.getProperty("rivet.ffi.tmpdir");
            Path dir = base == null
                    ? Files.createTempDirectory("rivetkit-ffi")
                    : Files.createTempDirectory(Path.of(base), "rivetkit-ffi");
            Path lib = dir.resolve(System.mapLibraryName("rivetkit_ffi"));
            Files.copy(in, lib, StandardCopyOption.REPLACE_EXISTING);
            // deleteOnExit runs in reverse registration order: register the directory
            // first so the library inside it is deleted before it.
            dir.toFile().deleteOnExit();
            lib.toFile().deleteOnExit();
            System.load(lib.toString());
        } catch (IOException e) {
            throw new UncheckedIOException("extract rivetkit-ffi", e);
        }
    }

    static String platform() {
        String os = System.getProperty("os.name").toLowerCase(Locale.ROOT);
        String osName = os.contains("linux") ? "linux" : os.contains("mac") ? "macos" : os.contains("windows") ? "windows" : os;
        String arch = switch (System.getProperty("os.arch")) {
            case "amd64", "x86_64" -> "x86_64";
            case "aarch64", "arm64" -> "aarch64";
            default -> System.getProperty("os.arch");
        };
        return osName + "-" + arch;
    }
}
