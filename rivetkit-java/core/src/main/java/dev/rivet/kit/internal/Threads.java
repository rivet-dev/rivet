package dev.rivet.kit.internal;

import java.util.concurrent.Executor;
import java.util.concurrent.ThreadFactory;

/** Thread helpers. All SDK work runs on virtual threads. */
public final class Threads {
    private Threads() {}

    private static final ThreadFactory FACTORY = Thread.ofVirtual().name("rivet-", 0).factory();

    public static final Executor VIRTUAL = command -> FACTORY.newThread(command).start();

    public static Thread start(String name, Runnable task) {
        return Thread.ofVirtual().name(name).start(task);
    }
}
