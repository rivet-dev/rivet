import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
	Component,
	type ReactNode,
	StrictMode,
	useEffect,
	useMemo,
	useState,
} from "react";
import ReactDOM from "react-dom/client";
import { createDefaultGlobalContext } from "@/app/data-providers/default-data-provider";
import { ActorInspectorProvider } from "@/components/actors/actor-inspector-context";
import { DataProviderContext } from "@/components/actors/data-provider";
import {
	InspectorTabContent,
	useAvailableInspectorTabs,
} from "@/components/actors/inspector-tab-registry";
import type { ActorId } from "@/components/actors/queries";
import { TooltipProvider } from "@/components/ui/tooltip";
import "@/index.css";
import { getInitialActorId, type InitMessage } from "./bridge";
import { BridgeClient } from "./bridge-client";

// Top-level error boundary. Cross-origin parents can't see inside us, so a
// blank iframe is a debugging dead end — surface the error in-place instead.
class IframeErrorBoundary extends Component<
	{ children: ReactNode },
	{ error: Error | null }
> {
	state = { error: null as Error | null };
	static getDerivedStateFromError(error: Error) {
		return { error };
	}
	componentDidCatch(error: Error) {
		console.error("Inspector UI crashed", error);
	}
	render() {
		if (!this.state.error) return this.props.children;
		return (
			<pre
				data-inspector-error=""
				style={{
					color: "#f55",
					padding: 16,
					whiteSpace: "pre-wrap",
					fontFamily:
						"ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace",
					fontSize: 12,
					margin: 0,
				}}
			>
				{`Inspector UI crashed:\n${this.state.error.message}\n\n${this.state.error.stack ?? ""}`}
			</pre>
		);
	}
}

// Mounted after init: publishes `tabs-available` to the shell as soon as we
// know the actor's capabilities, and renders the currently active tab's
// content. The tab strip lives in the dashboard, not here.
function InspectorContent({
	actorId,
	activeTab,
	bridge,
}: {
	actorId: ActorId;
	activeTab: string | undefined;
	bridge: BridgeClient;
}) {
	const availableTabs = useAvailableInspectorTabs(actorId);

	useEffect(() => {
		if (availableTabs) bridge.sendTabsAvailable(availableTabs);
	}, [bridge, availableTabs]);

	return <InspectorTabContent actorId={actorId} activeTab={activeTab} />;
}

function InspectorApp({
	actorId,
	credentials,
	bridge,
	activeTab,
}: {
	actorId: ActorId;
	credentials: { url: string; inspectorToken: string; token: string };
	bridge: BridgeClient;
	activeTab: string | undefined;
}) {
	const queryClient = useMemo(
		() =>
			new QueryClient({
				defaultOptions: {
					queries: { retry: false, staleTime: Infinity },
				},
			}),
		[],
	);

	// Stub data provider — the dashboard previously supplied this via the
	// router. The default context provides the no-fetch selectors that
	// inspector tabs read from the WS-populated cache.
	const dataProvider = useMemo(() => createDefaultGlobalContext(), []);

	return (
		<QueryClientProvider client={queryClient}>
			<TooltipProvider>
				<DataProviderContext.Provider
					value={
						dataProvider as React.ContextType<
							typeof DataProviderContext
						>
					}
				>
					<ActorInspectorProvider
						actorId={actorId}
						credentials={credentials}
					>
						<InspectorContent
							actorId={actorId}
							activeTab={activeTab}
							bridge={bridge}
						/>
					</ActorInspectorProvider>
				</DataProviderContext.Provider>
			</TooltipProvider>
		</QueryClientProvider>
	);
}

function BootGate({ bridge }: { bridge: BridgeClient }) {
	const actorIdFromUrl = useMemo(() => getInitialActorId(), []);
	const [init, setInit] = useState<InitMessage | null>(null);
	const [activeTab, setActiveTab] = useState<string | undefined>(undefined);

	useEffect(() => {
		bridge.start();
		const unsubInit = bridge.onInit((msg) => {
			setInit(msg);
			// Seed activeTab from the first init; ignore activeTab on
			// subsequent inits (e.g. token refresh) so we don't clobber
			// any tab switches the shell sent in between.
			setActiveTab((current) => current ?? msg.activeTab);
		});
		const unsubSetTab = bridge.onSetActiveTab((msg) => {
			setActiveTab(msg.tab);
		});
		bridge.sendReady();
		return () => {
			unsubInit();
			unsubSetTab();
			bridge.stop();
		};
	}, [bridge]);

	if (!init) return null;

	if (actorIdFromUrl && init.actorId !== actorIdFromUrl) {
		throw new Error(
			`Inspector init actorId (${init.actorId}) doesn't match URL actorId (${actorIdFromUrl})`,
		);
	}

	return (
		<InspectorApp
			actorId={init.actorId as ActorId}
			credentials={{
				url: window.location.origin,
				inspectorToken: init.authToken,
				token: init.rivetToken ?? "",
			}}
			bridge={bridge}
			activeTab={activeTab}
		/>
	);
}

const bridge = new BridgeClient();
const rootEl = document.getElementById("root");
if (!rootEl) throw new Error("Inspector UI: #root element missing");
ReactDOM.createRoot(rootEl).render(
	<StrictMode>
		<IframeErrorBoundary>
			<BootGate bridge={bridge} />
		</IframeErrorBoundary>
	</StrictMode>,
);
