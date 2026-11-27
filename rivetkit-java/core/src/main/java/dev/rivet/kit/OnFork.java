package dev.rivet.kit;

import java.util.List;

/** First start of a forked actor, after {@link OnCreate}: drop or adjust what the fork inherited. */
public interface OnFork {
    void onFork(Fork fork) throws Exception;

    /** Where this actor was forked from. {@code input} is the fork's CBOR input, or null. */
    record Fork(String parentActorId, List<String> parentKey, long forkedAt, byte[] input) {}
}
