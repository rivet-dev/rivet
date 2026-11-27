package dev.rivet.examples;

import dev.rivet.kit.Registry;

/**
 * Serves the examples. Reads RIVET_ENDPOINT, RIVET_TOKEN, RIVET_NAMESPACE, RIVET_POOL
 * from the environment and spawns a local engine when none is running.
 */
public final class Main {
    private Main() {}

    public static void main(String[] args) {
        Registry.Serving serving = Examples.registry()
                .serve(Registry.ServeOptions.fromEnv().withEngineSpawn(Registry.EngineSpawn.AUTO));
        Runtime.getRuntime().addShutdownHook(new Thread(serving::close));
        serving.ready().join();
        System.out.println("serving " + Examples.registry().names());
        serving.done().join();
    }
}
