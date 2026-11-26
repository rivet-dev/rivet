package dev.rivet.kit;

/**
 * First start only, after the constructor and before {@link OnWake}.
 *
 * @param <I> the create input type (what {@code Handle.withInput} sends); {@link None} for none
 */
public interface OnCreate<I> {
    void onCreate(I input) throws Exception;
}
