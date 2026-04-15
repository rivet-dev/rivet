import { StrictMode } from "react";
import ReactDOM from "react-dom/client";
import { ActorQueueTab } from "@/components/actors/actor-queue-tab";
import { TabRuntime, getActorIdFromUrl } from "../../src/tab-runtime";

const actorId = getActorIdFromUrl();

// biome-ignore lint/style/noNonNullAssertion: always present
const rootElement = document.getElementById("root")!;
if (!rootElement.innerHTML) {
	const root = ReactDOM.createRoot(rootElement);
	root.render(
		<StrictMode>
			<TabRuntime actorId={actorId}>
				<ActorQueueTab actorId={actorId} />
			</TabRuntime>
		</StrictMode>,
	);
}
