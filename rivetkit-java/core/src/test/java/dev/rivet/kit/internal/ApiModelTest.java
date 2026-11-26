package dev.rivet.kit.internal;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import dev.rivet.kit.Actor;
import dev.rivet.kit.Name;
import dev.rivet.kit.None;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;

class ApiModelTest {
    public interface ChatRoom extends Actor<ChatRoom.Event, ChatRoom.Params> {
        Message send(String text);

        @Name("recent")
        List<Message> history(int limit);

        default String helper() {
            return "client-side";
        }

        record Params(String user) {}

        record Message(String text) {}

        sealed interface Event {}

        record NewMessage(Message message) implements Event {}

        @Name("joined")
        record UserJoined(String user) implements Event {}
    }

    public interface Overloaded extends Actor<None, None> {
        void a(int x);

        void a(String x);
    }

    public interface IntKeys extends Actor<None, None> {
        Map<Integer, String> lookup();
    }

    public interface Generic extends Actor<None, None> {
        <T> T any();
    }

    public interface BadEvents extends Actor<String, None> {}

    @Test
    void readsActionsEventsAndParams() {
        ApiModel model = ApiModel.of(ChatRoom.class);
        assertEquals("chatRoom", model.name);
        assertEquals(java.util.Set.of("send", "recent"), model.actions.keySet());
        assertEquals(Set_of("newMessage", "joined"), java.util.Set.copyOf(model.eventsByName.keySet()));
        assertEquals(ChatRoom.Params.class, model.paramsType.getRawClass());
    }

    private static java.util.Set<String> Set_of(String... names) {
        return java.util.Set.of(names);
    }

    @Test
    void rejectsShapesThatBreakOnTheWire() {
        assertTrue(assertThrows(IllegalArgumentException.class, () -> ApiModel.of(Overloaded.class))
                .getMessage().contains("overloads"));
        assertTrue(assertThrows(IllegalArgumentException.class, () -> ApiModel.of(IntKeys.class))
                .getMessage().contains("String or enum keys"));
        assertTrue(assertThrows(IllegalArgumentException.class, () -> ApiModel.of(Generic.class))
                .getMessage().contains("generic"));
        assertTrue(assertThrows(IllegalArgumentException.class, () -> ApiModel.of(BadEvents.class))
                .getMessage().contains("sealed interface of records"));
    }

    @Test
    void recorderNamesTheAction() {
        ApiModel model = ApiModel.of(ChatRoom.class);
        Recorder.Recorded call = Recorder.record(model, p -> ((ChatRoom) p).history(5));
        assertEquals("recent", call.action().name);
        assertEquals(5, call.args()[0]);
        assertThrows(IllegalArgumentException.class, () -> Recorder.record(model, p -> ((ChatRoom) p).helper()));
        assertThrows(IllegalArgumentException.class, () -> Recorder.record(model, p -> ((Overloaded) p).a(1)));
    }
}
