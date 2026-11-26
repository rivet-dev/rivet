package dev.rivet.examples;

import dev.rivet.kit.Actor;
import dev.rivet.kit.None;

/** Raw HTTP with streaming bodies: POST /upload counts bytes as they arrive; GET /download?n= streams n bytes. */
public interface Files extends Actor<None, None> {
    long uploadedBytes();
}
