package dev.rivet.kit;

/** A connection opened. */
public interface OnConnect<P> {
    void onConnect(ActorConn<P> conn) throws Exception;
}
