import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
	createMemoryHistory,
	createRouter,
	RouterProvider,
} from "@tanstack/react-router";
import { useLayoutEffect, useState } from "react";
import rivetLogo from "./assets/rivet.svg";
import { TooltipProvider } from "./components";
import { routeTree } from "./routeTree.gen";

declare module "@tanstack/react-router" {
	interface Register {
		router: typeof router;
	}
}

const memoryHistory = createMemoryHistory({
	initialEntries: ["/"], // Pass your initial url
});
const getRoot = () => document.getElementById("rivetkit-inspector");
const getBody = () => document.body;
const queryClient = new QueryClient();
const router = createRouter({
	basepath: import.meta.env.BASE_URL,
	routeTree,
	context: {
		queryClient: queryClient,
	},
	defaultPreloadStaleTime: 0,
	defaultGcTime: 0,
	defaultPreloadGcTime: 0,
	defaultStaleTime: Infinity,
	scrollRestoration: true,
	defaultPendingMinMs: 300,
	history: memoryHistory,
	defaultOnCatch: (error) => {
		console.error("Router caught an error:", error);
	},
});

function App() {
	const [isOpen, setIsOpen] = useState(true);

	useLayoutEffect(() => {
		getRoot()?.style.setProperty("position", "fixed");
		getRoot()?.style.setProperty("z-index", "2147483647");
		if (isOpen) {
			getRoot()?.style.setProperty("bottom", "0");
			getRoot()?.style.setProperty("right", "0");
			getRoot()?.style.setProperty("left", "0");
			getBody()?.style.setProperty("padding-bottom", "24rem");
		} else {
			getRoot()?.style.setProperty("bottom", "16px");
			getRoot()?.style.setProperty("right", "16px");
			getRoot()?.style.removeProperty("left");
			getBody()?.style.removeProperty("padding-bottom");
		}
	}, [isOpen]);

	if (!isOpen) {
		return (
			<button
				type="button"
				className="size-10 block"
				onClick={() => setIsOpen((prev) => !prev)}
			>
				<img src={rivetLogo} className="w-full h-full" />
			</button>
		);
	}

	return (
		<div className="w-full h-96 bg-background-main border-t [&_*]:border-border [&_*]:text-foreground border-border">
			<QueryClientProvider client={queryClient}>
				<TooltipProvider>
					<RouterProvider router={router} />
				</TooltipProvider>
			</QueryClientProvider>
		</div>
	);
}

export default App;
