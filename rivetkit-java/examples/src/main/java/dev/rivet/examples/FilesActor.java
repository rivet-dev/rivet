package dev.rivet.examples;

import dev.rivet.kit.ActorContext;
import dev.rivet.kit.None;
import dev.rivet.kit.OnRequest;
import dev.rivet.kit.Request;
import dev.rivet.kit.Response;
import java.io.InputStream;
import java.util.Arrays;
import java.util.Map;

public final class FilesActor implements Files, OnRequest {
    private long uploaded;

    public FilesActor(ActorContext<None> ctx) {}

    @Override
    public Response onRequest(Request request) throws Exception {
        if (request.method().equals("POST") && request.path().endsWith("/upload")) {
            long total = 0;
            byte[] buffer = new byte[16 * 1024];
            try (InputStream body = request.body()) {
                for (int n; (n = body.read(buffer)) >= 0; ) total += n;
            }
            uploaded += total;
            return Response.text(200, Long.toString(total));
        }
        if (request.method().equals("GET") && request.path().endsWith("/download")) {
            long size = Long.parseLong(request.uri().replaceAll(".*[?&]n=(\\d+).*", "$1"));
            return Response.streaming(200, Map.of("content-type", "application/octet-stream"), out -> {
                byte[] chunk = new byte[8192];
                Arrays.fill(chunk, (byte) 'x');
                for (long left = size; left > 0; left -= chunk.length) {
                    out.write(chunk, 0, (int) Math.min(chunk.length, left));
                }
            });
        }
        return Response.text(404, "not found");
    }

    @Override
    public long uploadedBytes() {
        return uploaded;
    }
}
