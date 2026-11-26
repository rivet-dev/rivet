package dev.rivet.kit;

/** The actor is being destroyed. Runs after earlier events, cut off at the shutdown deadline. */
public interface OnDestroy {
    void onDestroy() throws Exception;
}
