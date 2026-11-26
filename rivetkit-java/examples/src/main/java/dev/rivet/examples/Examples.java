package dev.rivet.examples;

import dev.rivet.kit.Registry;

/** Every example actor in one registry. */
public final class Examples {
    private Examples() {}

    public static Registry registry() {
        return Registry.builder()
                .actor(Counter.class, CounterActor::new, actor -> actor.migrate(CounterActor::migrate))
                .actor(ChatRoom.class, ChatRoomActor::new, actor -> actor.migrate(ChatRoomActor::migrate))
                .actor(Files.class, FilesActor::new)
                .actor(Echo.class, EchoActor::new)
                .actor(Pinger.class, PingerActor::new, actor -> actor.migrate(PingerActor::migrate))
                .build();
    }
}
