package dev.rivet.kit;

/** The actor is going to sleep. Runs after earlier events, cut off at the shutdown deadline. */
public interface OnSleep {
    void onSleep() throws Exception;
}
