package dev.rivet.examples;

import dev.rivet.kit.Actor;
import dev.rivet.kit.Name;
import java.util.List;

/** A test actor that exercises the rest of the actor-side API. */
public interface Lab extends Actor<Lab.Event, Lab.Params> {
    List<String> info();

    Row sqlTypes();

    long rowsMatching(String label);

    void badParam();

    long scalarFromTwoColumns();

    long transfer(boolean fail);

    List<Long> batch();

    int schedules();

    List<String> crons();

    String keepAwake();

    int spawnAndWait() throws Exception;

    String blocking();

    String exclusive();

    String touchCtxFromForeignThread() throws Exception;

    int pushToCaller(String text);

    void kickCaller();

    int disconnects();

    String createdWith();

    long sleeps();

    void sleepNow();

    void destroyNow();

    void userErrorWithMetadata();

    void crash();

    long add2(long a, long b);

    long add3(long a, long b, long c);

    void store(String key, String value);

    void store3(String key, String value, String suffix);

    String stored(String key);

    int socketsClosed();

    /** Subscribes this actor to a counter's events; the listener runs as turns of this actor. */
    void watchCounter(String counterKey);

    long lastSeenCount();

    record Params(String user) {}

    record CreateInput(String note) {}

    enum Color { RED, GREEN }

    /** Read back through SQL: column names are snake_case. */
    record Row(long intValue, double floatValue, boolean flag, String text, byte[] blob, Color color, String missing) {}

    sealed interface Event {}

    record Pushed(String text) implements Event {}

    @Name("secret")
    record Secret(String value) implements Event {}
}
