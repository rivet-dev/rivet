package dev.rivet.kit.internal;

import static java.lang.foreign.ValueLayout.JAVA_BYTE;

import dev.rivet.kit.RivetException;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.lang.foreign.MemorySegment;
import java.util.ArrayDeque;

/** HTTP bodies over the wire's streams. */
final class Streams {
    private Streams() {}

    /**
     * An inbound stream. Chunks arrive leased: {@link #read} copies straight from
     * Rust's buffer into the caller's array, then releases the lease, which returns
     * credit so Rust sends more. A reader that stops reading pauses the sender.
     */
    static final class Inbound extends InputStream implements Native.ResourceSink {
        private static final class Chunk {
            final MemorySegment data;
            final long lease;
            long offset;

            Chunk(MemorySegment data, long lease) {
                this.data = data;
                this.lease = lease;
            }
        }

        private final long id;
        private final ArrayDeque<Chunk> chunks = new ArrayDeque<>();
        private boolean ended;
        private boolean closed;
        private RivetException error;

        Inbound(long id) {
            this.id = id;
            Native.registerResource(id, this);
        }

        @Override
        public synchronized void onChunk(MemorySegment data, long lease) {
            if (closed) {
                Native.leaseRelease(lease);
                return;
            }
            chunks.addLast(new Chunk(data, lease));
            notifyAll();
        }

        @Override
        public synchronized void onEvent(int kind, byte[] header, byte[] body) {
            if (kind != Wire.STREAM_END) return;
            Wire.StreamEndEvent end = Codec.decode(header, Wire.StreamEndEvent.class);
            if (end.error() != null) {
                error = new RivetException(end.error().group(), end.error().code(), end.error().message());
            }
            ended = true;
            Native.unregisterResource(id);
            notifyAll();
        }

        @Override
        public int read() throws IOException {
            byte[] one = new byte[1];
            int n = read(one, 0, 1);
            return n < 0 ? -1 : one[0] & 0xff;
        }

        @Override
        public synchronized int read(byte[] b, int off, int len) throws IOException {
            if (len == 0) return 0;
            while (chunks.isEmpty()) {
                if (closed) throw new IOException("stream closed");
                if (ended) {
                    if (error != null) throw new IOException("body aborted: " + error.getMessage(), error);
                    return -1;
                }
                try {
                    wait();
                } catch (InterruptedException e) {
                    Thread.currentThread().interrupt();
                    throw new IOException("interrupted while reading a body", e);
                }
            }
            Chunk chunk = chunks.peekFirst();
            int n = (int) Math.min(len, chunk.data.byteSize() - chunk.offset);
            MemorySegment.copy(chunk.data, JAVA_BYTE, chunk.offset, b, off, n);
            chunk.offset += n;
            if (chunk.offset == chunk.data.byteSize()) {
                chunks.pollFirst();
                Native.leaseRelease(chunk.lease);
            }
            return n;
        }

        @Override
        public synchronized int available() {
            Chunk chunk = chunks.peekFirst();
            return chunk == null ? 0 : (int) (chunk.data.byteSize() - chunk.offset);
        }

        @Override
        public synchronized void close() {
            if (closed) return;
            closed = true;
            for (Chunk chunk : chunks) Native.leaseRelease(chunk.lease);
            chunks.clear();
            if (!ended) {
                Native.unregisterResource(id);
                Native.release(id);
            }
            notifyAll();
        }
    }

    /**
     * A streamed response body. Each write waits until Rust accepts the bytes (the
     * client's backpressure) and gives up the actor's turn while it waits.
     */
    static final class Outbound extends OutputStream {
        private final long id;
        private boolean closed;

        Outbound(long id) {
            this.id = id;
        }

        @Override
        public void write(int b) throws IOException {
            write(new byte[] {(byte) b}, 0, 1);
        }

        @Override
        public void write(byte[] b, int off, int len) throws IOException {
            if (closed) throw new IOException("response body is closed");
            if (len == 0) return;
            byte[] data = off == 0 && len == b.length ? b : java.util.Arrays.copyOfRange(b, off, off + len);
            send(data, false);
        }

        private void send(byte[] data, boolean fin) throws IOException {
            try {
                Calls.outbound(() -> Native.callSync(id, Wire.OP_STREAM_WRITE, new Wire.StreamWriteCall(fin), data));
            } catch (RivetException e) {
                closed = true;
                throw new IOException("client stopped reading: " + e.getMessage(), e);
            }
        }

        @Override
        public void close() throws IOException {
            if (closed) return;
            send(new byte[0], true);
            closed = true;
        }

        void abort(String message) {
            if (closed) return;
            closed = true;
            try {
                Native.callSync(id, Wire.OP_STREAM_ERROR, new Wire.StreamErrorCall(message), null);
            } catch (RivetException ignored) {
                // already gone
            }
        }
    }
}
