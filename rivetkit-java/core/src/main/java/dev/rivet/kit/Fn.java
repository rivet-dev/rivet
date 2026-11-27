package dev.rivet.kit;

/**
 * Method-reference shapes for typed calls by reference, such as
 * {@code ctx.schedule().after(d, ChatRoom::purge, cutoff)} or
 * {@code handle.async(Counter::increment, 5L)}. {@code A} is the actor API.
 */
public final class Fn {
    private Fn() {}

    @FunctionalInterface
    public interface Call0<A> {
        void call(A api) throws Exception;
    }

    @FunctionalInterface
    public interface Call1<A, T1> {
        void call(A api, T1 a1) throws Exception;
    }

    @FunctionalInterface
    public interface Call2<A, T1, T2> {
        void call(A api, T1 a1, T2 a2) throws Exception;
    }

    @FunctionalInterface
    public interface Call3<A, T1, T2, T3> {
        void call(A api, T1 a1, T2 a2, T3 a3) throws Exception;
    }

    @FunctionalInterface
    public interface Get0<A, R> {
        R call(A api) throws Exception;
    }

    @FunctionalInterface
    public interface Get1<A, T1, R> {
        R call(A api, T1 a1) throws Exception;
    }

    @FunctionalInterface
    public interface Get2<A, T1, T2, R> {
        R call(A api, T1 a1, T2 a2) throws Exception;
    }

    @FunctionalInterface
    public interface Get3<A, T1, T2, T3, R> {
        R call(A api, T1 a1, T2 a2, T3 a3) throws Exception;
    }
}
