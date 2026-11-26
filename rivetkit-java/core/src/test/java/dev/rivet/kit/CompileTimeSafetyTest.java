package dev.rivet.kit;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.StringWriter;
import java.net.URI;
import java.util.List;
import java.util.stream.Stream;
import javax.tools.JavaCompiler;
import javax.tools.JavaFileObject;
import javax.tools.SimpleJavaFileObject;
import javax.tools.ToolProvider;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.Arguments;
import org.junit.jupiter.params.provider.MethodSource;

/**
 * The API's promise: these mistakes do not compile. Each case is compiled with javac
 * against the real SDK; the good case must compile and every bad case must not.
 */
class CompileTimeSafetyTest {
    private static final String API = """
            import dev.rivet.kit.*;
            import java.time.Duration;
            interface ChatRoom extends Actor<ChatRoom.Event, ChatRoom.Params> {
                Message send(String text);
                void purge(long cutoff);
                record Params(String user) {}
                record Message(String text) {}
                sealed interface Event {}
                record NewMessage(Message m) implements Event {}
            }
            interface Counter extends Actor<None, None> { long increment(long by); }
            record Other(int x) {}
            """;

    static Stream<Arguments> cases() {
        return Stream.of(
                Arguments.of("good", true, """
                        class ChatRoomActor implements ChatRoom, OnConnect<ChatRoom.Params> {
                            final ActorContext<Event> ctx;
                            ChatRoomActor(ActorContext<Event> ctx) { this.ctx = ctx; }
                            public Message send(String t) { ctx.broadcast(new NewMessage(new Message(t)));
                                ctx.schedule().after(Duration.ZERO, ChatRoom::purge, 1L); return null; }
                            public void purge(long c) {}
                            public void onConnect(ActorConn<Params> c) {}
                        }
                        class Use { void f(Client c) throws Exception {
                            Registry.builder().actor(ChatRoom.class, ChatRoomActor::new).build();
                            var conn = c.connect(ChatRoom.class, () -> new ChatRoom.Params("a"), "k");
                            conn.on(ChatRoom.NewMessage.class, e -> {});
                            long n = c.handle(Counter.class, "k").async(Counter::increment, 1L).get();
                        } }
                        """),
                Arguments.of("broadcast a non-event", false,
                        "class X { void f(ActorContext<ChatRoom.Event> ctx) { ctx.broadcast(new Other(1)); } }"),
                Arguments.of("broadcast from an actor without events", false,
                        "class X { void f(ActorContext<None> ctx) { ctx.broadcast(\"x\"); } }"),
                Arguments.of("subscribe to another type", false,
                        "class X { void f(Client c) { c.connect(ChatRoom.class, \"k\").on(Other.class, e -> {}); } }"),
                Arguments.of("wrong params type", false,
                        "class X { void f(Client c) { c.connect(ChatRoom.class, () -> \"nope\", \"k\"); } }"),
                Arguments.of("misspelled hook", false, """
                        class X implements Counter, OnConnect<None> {
                            public long increment(long b) { return 0; }
                            public void onConect(ActorConn<None> c) {}
                        }
                        """),
                Arguments.of("schedule with a wrong argument", false,
                        "class X { void f(ActorContext<ChatRoom.Event> ctx) { ctx.schedule().after(java.time.Duration.ZERO, ChatRoom::purge, \"x\"); } }"),
                Arguments.of("async with a wrong argument", false,
                        "class X { void f(Client c) { c.handle(Counter.class, \"k\").async(Counter::increment, \"x\"); } }"),
                Arguments.of("async on another actor's method", false,
                        "class X { void f(Client c) { c.handle(Counter.class, \"k\").async(ChatRoom::send, \"x\"); } }"),
                Arguments.of("implementation of another API", false, """
                        class CounterActor implements Counter { CounterActor(ActorContext<None> c) {} public long increment(long b) { return b; } }
                        class X { void f() { Registry.builder().actor(ChatRoom.class, CounterActor::new); } }
                        """),
                Arguments.of("not an actor API", false,
                        "class X { interface Plain { void x(); } void f(Client c) { c.getOrCreate(Plain.class, \"k\"); } }"));
    }

    @ParameterizedTest(name = "{0}")
    @MethodSource("cases")
    void compiles(String name, boolean shouldCompile, String code) {
        JavaCompiler javac = ToolProvider.getSystemJavaCompiler();
        StringWriter out = new StringWriter();
        JavaFileObject source = new SimpleJavaFileObject(URI.create("string:///Case.java"), JavaFileObject.Kind.SOURCE) {
            @Override
            public CharSequence getCharContent(boolean ignoreEncodingErrors) {
                return API + code;
            }
        };
        List<String> options = List.of("-proc:none", "-d", System.getProperty("java.io.tmpdir") + "/rivet-compile-safety",
                "-cp", System.getProperty("java.class.path"));
        boolean ok = javac.getTask(out, null, null, options, null, List.of(source)).call();
        if (shouldCompile) assertTrue(ok, name + " should compile:\n" + out);
        else assertEquals(false, ok, name + " should not compile");
    }
}
