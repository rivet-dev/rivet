import { StrictMode } from "react";
import ReactDOM from "react-dom/client";
import { ActorConsoleFull } from "@/components/actors/console/actor-console";
import { ActorWorkerContextProvider } from "@/components/actors/worker/actor-worker-context";
import { TabRuntime, getActorIdFromUrl } from "../../src/tab-runtime";

const actorId = getActorIdFromUrl();

// biome-ignore lint/style/noNonNullAssertion: always present
const rootElement = document.getElementById("root")!;
if (!rootElement.innerHTML) {
	const root = ReactDOM.createRoot(rootElement);
	root.render(
		<StrictMode>
			<TabRuntime actorId={actorId}>
				<ActorWorkerContextProvider actorId={actorId}>
					<ActorConsoleFull actorId={actorId} />
				</ActorWorkerContextProvider>
			</TabRuntime>
		</StrictMode>,
	);
}
