import { broadcastQueryClient } from "@tanstack/query-broadcast-client-experimental";
import { useQueryClient } from "@tanstack/react-query";
import {
	createContext,
	type ReactNode,
	useCallback,
	useContext,
	useEffect,
	useRef,
} from "react";
import { useActorInspector } from "./actor-inspector-context";
import type { ActorId } from "./queries";

interface IframeTabBridgeContextValue {
	registerIframe: (tabId: string, iframe: HTMLIFrameElement) => void;
	unregisterIframe: (tabId: string) => void;
}

const IframeTabBridgeContext = createContext<IframeTabBridgeContextValue>({
	registerIframe: () => {},
	unregisterIframe: () => {},
});

export const useIframeTabBridge = () => useContext(IframeTabBridgeContext);

// Origins we accept action-request messages from (same-origin tab iframes).
const ALLOWED_TAB_ORIGINS = new Set([window.location.origin]);

// Actions the bridge is willing to dispatch to the inspector API.
const ALLOWED_ACTIONS = new Set([
	"ping",
	"executeAction",
	"patchState",
	"getConnections",
	"getState",
	"getRpcs",
	"getTraces",
	"getQueueStatus",
	"getWorkflowHistory",
	"replayWorkflowFromStep",
	"getDatabaseSchema",
	"getDatabaseTableRows",
	"executeDatabaseSql",
	"getMetadata",
]);

function getIframeOrigin(iframe: HTMLIFrameElement): string {
	try {
		return new URL(iframe.src).origin;
	} catch {
		return window.location.origin;
	}
}

/**
 * Provides the shell-side postMessage bridge for inspector tab iframes.
 *
 * Responsibilities:
 * - Sets up broadcastQueryClient so same-origin iframes receive cache updates
 *   automatically via BroadcastChannel.
 * - For cross-origin iframes, subscribes to the query cache and forwards
 *   individual updates as query-update postMessages.
 * - Listens for action-request messages from iframes, dispatches them to the
 *   real ActorInspectorApi, and sends back action-response.
 * - Sends an init snapshot to each iframe on load and when it signals ready.
 */
export function IframeTabBridgeProvider({
	actorId,
	children,
}: {
	actorId: ActorId;
	children: ReactNode;
}) {
	const queryClient = useQueryClient();
	const inspector = useActorInspector();
	const iframeMap = useRef(new Map<string, HTMLIFrameElement>());

	// Keep a stable ref to the latest inspector so event handlers always see the
	// current API without needing to re-register listeners on every render.
	const inspectorRef = useRef(inspector);
	inspectorRef.current = inspector;

	// Set up broadcastQueryClient once so same-origin tab iframes get cache
	// updates automatically through BroadcastChannel.
	useEffect(() => {
		broadcastQueryClient({
			queryClient,
			broadcastChannel: `rivetkit-inspector-${actorId}`,
		});
	}, [queryClient, actorId]);

	// Build a query cache snapshot for the init message.
	const getQueryCacheSnapshot = useCallback((): Record<string, unknown> => {
		const result: Record<string, unknown> = {};
		for (const query of queryClient.getQueryCache().getAll()) {
			const keyStr = JSON.stringify(query.queryKey);
			if (keyStr.includes(actorId)) {
				result[keyStr] = query.state.data;
			}
		}
		return result;
	}, [queryClient, actorId]);

	// Send the init snapshot to a specific iframe.
	const sendInit = useCallback(
		(iframe: HTMLIFrameElement) => {
			const iframeOrigin = getIframeOrigin(iframe);
			iframe.contentWindow?.postMessage(
				{
					type: "init",
					v: 1,
					actorId,
					queryCache: getQueryCacheSnapshot(),
					features: inspectorRef.current.features,
					rivetkitVersion: inspectorRef.current.rivetkitVersion,
					inspectorProtocolVersion:
						inspectorRef.current.inspectorProtocolVersion,
				},
				iframeOrigin,
			);
		},
		[actorId, getQueryCacheSnapshot],
	);

	// Forward cross-origin cache updates via postMessage.
	useEffect(() => {
		const unsubscribe = queryClient.getQueryCache().subscribe((event) => {
			if (event.type !== "updated" && event.type !== "added") return;
			const { queryKey } = event.query;
			const keyStr = JSON.stringify(queryKey);
			if (!keyStr.includes(actorId)) return;

			for (const iframe of iframeMap.current.values()) {
				const iframeOrigin = getIframeOrigin(iframe);
				if (iframeOrigin === window.location.origin) continue;
				// Cross-origin iframe: push the update via postMessage.
				iframe.contentWindow?.postMessage(
					{
						type: "query-update",
						v: 1,
						queryKey,
						data: event.query.state.data,
					},
					iframeOrigin,
				);
			}
		});
		return unsubscribe;
	}, [queryClient, actorId]);

	// Handle action-request and ready messages from tab iframes.
	useEffect(() => {
		const handleMessage = async (event: MessageEvent) => {
			if (!ALLOWED_TAB_ORIGINS.has(event.origin)) return;
			const msg = event.data;
			if (!msg?.type || msg.v !== 1) return;

			const source = event.source as Window;

			if (msg.type === "ready") {
				// Find which iframe signalled ready and send the init snapshot.
				for (const iframe of iframeMap.current.values()) {
					if (iframe.contentWindow === source) {
						sendInit(iframe);
						break;
					}
				}
				return;
			}

			if (msg.type === "action-request") {
				const { requestId, action } = msg as {
					requestId: string;
					action: { name: string; args: unknown[] };
				};

				if (!ALLOWED_ACTIONS.has(action.name)) {
					source.postMessage(
						{
							type: "action-response",
							v: 1,
							requestId,
							error: `Unknown inspector action: ${action.name}`,
						},
						event.origin,
					);
					return;
				}

				try {
					const api =
						inspectorRef.current.api as unknown as Record<
							string,
							(...args: unknown[]) => Promise<unknown>
						>;
					const result = await api[action.name](...action.args);
					source.postMessage(
						{ type: "action-response", v: 1, requestId, result },
						event.origin,
					);
				} catch (err) {
					source.postMessage(
						{
							type: "action-response",
							v: 1,
							requestId,
							error: err instanceof Error ? err.message : String(err),
						},
						event.origin,
					);
				}
			}
		};

		window.addEventListener("message", handleMessage);
		return () => window.removeEventListener("message", handleMessage);
	}, [sendInit]);

	const registerIframe = useCallback(
		(tabId: string, iframe: HTMLIFrameElement) => {
			iframeMap.current.set(tabId, iframe);

			// If the iframe already loaded before registration, send init now.
			try {
				if (iframe.contentDocument?.readyState === "complete") {
					sendInit(iframe);
				}
			} catch {
				// Cross-origin iframe — rely on the ready postMessage instead.
			}
		},
		[sendInit],
	);

	const unregisterIframe = useCallback((tabId: string) => {
		iframeMap.current.delete(tabId);
	}, []);

	return (
		<IframeTabBridgeContext.Provider value={{ registerIframe, unregisterIframe }}>
			{children}
		</IframeTabBridgeContext.Provider>
	);
}
