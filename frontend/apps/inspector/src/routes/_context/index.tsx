import { createFileRoute } from "@tanstack/react-router";
import { InspectorContextProvider } from "@/app/inspector-context";
import { InspectorRoot } from "@/app/inspector-root";

export const Route = createFileRoute("/_context/")({
	component: () => (
		<InspectorContextProvider>
			<InspectorRoot />
		</InspectorContextProvider>
	),
});
