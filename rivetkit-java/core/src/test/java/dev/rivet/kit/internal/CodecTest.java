package dev.rivet.kit.internal;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;

import dev.rivet.kit.RivetException;
import java.lang.reflect.Type;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;

class CodecTest {
    public record Message(String sender, String text, long at) {}

    @Test
    void positionalRoundTrip() {
        Codec.Positional args = new Codec.Positional(new Type[] {long.class, String.class}, new String[] {"n", "s"});
        Object[] decoded = args.decode(args.encode(new Object[] {5L, "x"}));
        assertArrayEquals(new Object[] {5L, "x"}, decoded);
    }

    @Test
    void decodesWhatOtherClientsSend() {
        Codec.Positional args = new Codec.Positional(new Type[] {long.class}, new String[] {"amount"});
        // TypeScript: [5]
        assertEquals(5L, args.decode(Codec.encode(List.of(5)))[0]);
        // Rust's struct form: [{"amount": 5}]
        assertEquals(5L, args.decode(Codec.encode(List.of(Map.of("amount", 5))))[0]);
        // Extra trailing arguments are ignored, as in JavaScript.
        assertEquals(5L, args.decode(Codec.encode(List.of(5, 6)))[0]);
    }

    @Test
    void missingPrimitiveIsAnError() {
        Codec.Positional args = new Codec.Positional(new Type[] {long.class}, new String[] {"amount"});
        RivetException e = assertThrows(RivetException.class, () -> args.decode(Codec.encode(List.of())));
        assertEquals("invalid_args", e.code());
    }

    @Test
    void missingObjectIsNull() {
        Codec.Positional args = new Codec.Positional(new Type[] {String.class}, null);
        assertNull(args.decode(Codec.encode(List.of()))[0]);
    }

    @Test
    void recordsRoundTrip() {
        Codec.Positional args = new Codec.Positional(new Type[] {Message.class}, new String[] {"m"});
        Message m = new Message("a", "b", 3);
        assertEquals(m, args.decode(args.encode(new Object[] {m}))[0]);
    }

    @Test
    void eventsAreComponentsInOrder() {
        Codec.EventCodec codec = new Codec.EventCodec(Message.class, "message");
        byte[] bytes = codec.encode(new Message("a", "b", 3));
        assertEquals(List.of("a", "b", 3), Codec.MAPPER.readValue(bytes, List.class));
        assertEquals(new Message("a", "b", 3), codec.decode(bytes));
        // The Rust SDK sends one map argument.
        assertEquals(new Message("a", "b", 3),
                codec.decode(Codec.encode(List.of(Map.of("sender", "a", "text", "b", "at", 3)))));
    }
}
