package dev.rivet.kit.internal;

import dev.rivet.kit.ActorConn;

/** A connection to an actor, as the actor sees it. */
final class ConnImpl implements ActorConn<Object> {
    private final Instance instance;
    private final String id;
    private final Object params;

    ConnImpl(Instance instance, String id, Object params) {
        this.instance = instance;
        this.id = id;
        this.params = params;
    }

    @Override
    public String id() {
        return id;
    }

    @Override
    public Object params() {
        return params;
    }

    @Override
    public void disconnect(String reason) {
        Native.callSync(instance.handle, Wire.OP_CONN_DISCONNECT, new Wire.ConnDisconnectCall(id, reason), null);
    }

    @Override
    public String toString() {
        return "ActorConn[" + id + "]";
    }
}
