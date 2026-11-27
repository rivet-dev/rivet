package dev.rivet.kit;

/** Handles raw HTTP requests to the actor ({@code /request/...} on the gateway). */
public interface OnRequest {
    Response onRequest(Request request) throws Exception;
}
