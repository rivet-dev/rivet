import { ClientOnly, createFileRoute } from "@tanstack/react-router";
import { Counter } from "@/components/Counter";

export const Route = createFileRoute("/")({ component: App });

function App() {
	return (
		<div className="flex-1 flex items-center justify-center">
			<ClientOnly>
				<Counter />
			</ClientOnly>
		</div>
	);
}
