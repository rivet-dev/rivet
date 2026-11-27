package dev.rivet.kit;

/** Every start (first start and every wake from sleep), after the constructor. */
public interface OnWake {
    void onWake() throws Exception;
}
