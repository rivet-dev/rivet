import { StrictMode } from "react";
import ReactDOM from "react-dom/client";
import { ActorStateTab } from "@/components/actors/actor-state-tab";
import { TabRuntime, getActorIdFromUrl } from "../../src/tab-runtime";

const actorId = getActorIdFromUrl();

// biome-ignore lint/style/noNonNullAssertion: always present
const rootElement = document.getElementById("root")!;
if (!rootElement.innerHTML) {
	const root = ReactDOM.createRoot(rootElement);
	root.render(
		<StrictMode>
			<TabRuntime actorId={actorId}>
				<ActorStateTab actorId={actorId} />
			</TabRuntime>
		</StrictMode>,
	);
}
