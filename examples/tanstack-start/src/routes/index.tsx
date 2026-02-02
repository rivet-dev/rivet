import { ClientOnly, createFileRoute } from "@tanstack/react-router";
import "../App.css";
import { Counter } from "@/components/Counter";

export const Route = createFileRoute("/")({ component: App });

function App() {
	return (
		<div className="App">
			<ClientOnly>
				<Counter />
			</ClientOnly>
		</div>
	);
}
