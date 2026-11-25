import { createServer } from "node:http";
import { NodeHttpServer, NodeRuntime } from "@effect/platform-node";
import { Registry } from "@rivetkit/effect";
import { Layer } from "effect";
import { HttpRouter } from "effect/http";
import { CounterLive } from "../actors-quickstart-effect/src/live";

// Serves the metrics of this node on a private port.
const MetricsServer = HttpRouter.add(
	"GET",
	"/metrics",
	Registry.prometheusMetrics,
).pipe(
	HttpRouter.serve,
	Layer.provide(
		NodeHttpServer.layer(createServer, {
			host: "127.0.0.1",
			port: Number(process.env.RIVETKIT_METRICS_PORT ?? 9464),
		}),
	),
);

// Serves the app routes and the health of this node on the public port.
const AppServer = HttpRouter.add("GET", "/health", Registry.health).pipe(
	HttpRouter.serve,
	Layer.provide(
		NodeHttpServer.layer(createServer, {
			port: Number(process.env.PORT ?? 8080),
		}),
	),
);

const MainLayer = Layer.mergeAll(
	Registry.serve(CounterLive),
	MetricsServer,
	AppServer,
).pipe(
	Layer.provide(
		Registry.layer({
			startEngine: false,
			startServices: false,
			runtime: "native",
		}),
	),
);

Layer.launch(MainLayer).pipe(NodeRuntime.runMain);
