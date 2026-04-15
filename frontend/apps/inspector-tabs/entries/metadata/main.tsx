import { StrictMode } from "react";
import ReactDOM from "react-dom/client";
import { ActorConfigTab } from "@/components/actors/actor-config-tab";
import { TabRuntime, getActorIdFromUrl } from "../../src/tab-runtime";

const actorId = getActorIdFromUrl();

// biome-ignore lint/style/noNonNullAssertion: always present
const rootElement = document.getElementById("root")!;
if (!rootElement.innerHTML) {
	const root = ReactDOM.createRoot(rootElement);
	root.render(
		<StrictMode>
			<TabRuntime actorId={actorId}>
				<ActorConfigTab actorId={actorId} />
			</TabRuntime>
		</StrictMode>,
	);
}
