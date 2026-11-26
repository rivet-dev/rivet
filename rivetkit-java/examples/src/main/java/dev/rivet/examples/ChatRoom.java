package dev.rivet.examples;

import dev.rivet.kit.Actor;
import java.util.List;

/** A chat room: messages in SQLite, who is online in memory, events to everyone connected. */
public interface ChatRoom extends Actor<ChatRoom.Event, ChatRoom.Params> {
    Message send(String text);

    List<Message> history(int limit);

    /** Messages sent since this actor last woke up (an in-memory var). */
    int sentSinceWake();

    List<String> online();

    /** Deletes messages older than {@code cutoffMillis}; also the target of a cron job. */
    int purgeOlderThan(long cutoffMillis);

    void goToSleep();

    record Params(String user, String token) {}

    record Message(String sender, String text, long at) {}

    sealed interface Event {}

    record NewMessage(Message message) implements Event {}

    record Joined(String user) implements Event {}
}
