package dev.rivet.kit;

/** A connection closed. */
public interface OnDisconnect<P> {
    void onDisconnect(ActorConn<P> conn) throws Exception;
}
