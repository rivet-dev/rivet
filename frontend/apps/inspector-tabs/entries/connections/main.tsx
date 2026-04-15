import { StrictMode } from "react";
import ReactDOM from "react-dom/client";
import { ActorConnectionsTab } from "@/components/actors/actor-connections-tab";
import { TabRuntime, getActorIdFromUrl } from "../../src/tab-runtime";

const actorId = getActorIdFromUrl();

// biome-ignore lint/style/noNonNullAssertion: always present
const rootElement = document.getElementById("root")!;
if (!rootElement.innerHTML) {
	const root = ReactDOM.createRoot(rootElement);
	root.render(
		<StrictMode>
			<TabRuntime actorId={actorId}>
				<ActorConnectionsTab actorId={actorId} />
			</TabRuntime>
		</StrictMode>,
	);
}
