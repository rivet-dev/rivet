import { broadcastQueryClient } from "@tanstack/query-broadcast-client-experimental";
import type { QueryClient } from "@tanstack/react-query";
import {
	createContext,
	type ReactNode,
	useContext,
	useEffect,
	useMemo,
	useRef,
	useState,
} from "react";
import type { ActorId } from "@/components/actors/queries";

type FeatureSupport = {
	supported: boolean;
	minVersion: string;
	currentVersion?: string;
	message: string;
};

type InitMessage = {
	type: "init";
	v: 1;
	actorId: string;
	queryCache: Record<string, unknown>;
	features: { traces: FeatureSupport; queue: FeatureSupport };
	rivetkitVersion?: string;
	inspectorProtocolVersion?: number;
};

type QueryUpdateMessage = {
	type: "query-update";
	v: 1;
	queryKey: unknown[];
	data: unknown;
};

type ActionResponseMessage = {
	type: "action-response";
	v: 1;
	requestId: string;
	result?: unknown;
	error?: string;
};

type ShellToTabMessage = InitMessage | QueryUpdateMessage | ActionResponseMessage;

// Origins allowed to send messages to this tab.
const ALLOWED_SHELL_ORIGINS = new Set([
	window.location.origin,
	"https://dashboard.rivet.dev",
]);

interface TabContextValue {
	actorId: ActorId;
	queryClient: QueryClient;
	sendAction: (action: {
		name: string;
		args: unknown[];
	}) => Promise<unknown>;
	features: { traces: FeatureSupport; queue: FeatureSupport };
	rivetkitVersion?: string;
	inspectorProtocolVersion: number;
	isInitialized: boolean;
}

const TabContext = createContext<TabContextValue>({} as TabContextValue);

export const useTabContext = () => useContext(TabContext);

export function TabContextProvider({
	actorId,
	queryClient,
	children,
}: {
	actorId: ActorId;
	queryClient: QueryClient;
	children: ReactNode;
}) {
	const [features, setFeatures] = useState<{
		traces: FeatureSupport;
		queue: FeatureSupport;
	}>({
		traces: { supported: false, minVersion: "", message: "" },
		queue: { supported: false, minVersion: "", message: "" },
	});
	const [rivetkitVersion, setRivetkitVersion] = useState<string | undefined>();
	const [inspectorProtocolVersion, setInspectorProtocolVersion] = useState(0);
	const [isInitialized, setIsInitialized] = useState(false);

	const pendingActions = useRef(
		new Map<
			string,
			{
				resolve: (v: unknown) => void;
				reject: (e: Error) => void;
				timeoutId: ReturnType<typeof setTimeout>;
			}
		>(),
	);
	const shellOrigin = useRef<string | null>(null);
	const broadcastSetUp = useRef(false);

	useEffect(() => {
		// Signal readiness to the shell.
		window.parent.postMessage({ type: "ready", v: 1 }, "*");

		const handleMessage = (event: MessageEvent) => {
			const origin = event.origin;

			// Reject messages from untrusted origins.
			if (!ALLOWED_SHELL_ORIGINS.has(origin)) return;

			// Record the shell origin from the first valid message.
			if (!shellOrigin.current) {
				shellOrigin.current = origin;
			}

			const msg = event.data as ShellToTabMessage;
			if (!msg?.type || msg.v !== 1) return;

			if (msg.type === "init") {
				// Hydrate query cache from the snapshot sent by the shell.
				for (const [key, data] of Object.entries(msg.queryCache)) {
					try {
						const queryKey = JSON.parse(key) as unknown[];
						queryClient.setQueryData(queryKey, data);
					} catch {
						// Ignore malformed keys.
					}
				}

				setFeatures(msg.features);
				setRivetkitVersion(msg.rivetkitVersion);
				if (msg.inspectorProtocolVersion !== undefined) {
					setInspectorProtocolVersion(msg.inspectorProtocolVersion);
				}
				setIsInitialized(true);

				// For same-origin shells, set up broadcastQueryClient so future
				// cache updates arrive automatically without extra postMessages.
				if (!broadcastSetUp.current && origin === window.location.origin) {
					broadcastSetUp.current = true;
					broadcastQueryClient({
						queryClient,
						broadcastChannel: `rivetkit-inspector-${msg.actorId}`,
					});
				}
			} else if (msg.type === "query-update") {
				// Cross-origin shells send individual cache updates.
				queryClient.setQueryData(msg.queryKey as string[], msg.data);
			} else if (msg.type === "action-response") {
				const pending = pendingActions.current.get(msg.requestId);
				if (pending) {
					pendingActions.current.delete(msg.requestId);
					clearTimeout(pending.timeoutId);
					if (msg.error) {
						pending.reject(new Error(msg.error));
					} else {
						pending.resolve(msg.result);
					}
				}
			}
		};

		window.addEventListener("message", handleMessage);
		return () => {
			window.removeEventListener("message", handleMessage);
			// Reject all in-flight actions and clear their timeouts on unmount.
			for (const { reject, timeoutId } of pendingActions.current.values()) {
				clearTimeout(timeoutId);
				reject(new Error("Inspector context unmounted"));
			}
			pendingActions.current.clear();
		};
	}, [queryClient]);

	const sendAction = useMemo(
		() =>
			async (action: { name: string; args: unknown[] }): Promise<unknown> => {
				return new Promise<unknown>((resolve, reject) => {
					const requestId = crypto.randomUUID();
					const timeoutId = setTimeout(() => {
						if (pendingActions.current.has(requestId)) {
							pendingActions.current.delete(requestId);
							reject(new Error("Inspector action timed out"));
						}
					}, 30_000);
					pendingActions.current.set(requestId, { resolve, reject, timeoutId });

					const target = shellOrigin.current ?? "*";
					window.parent.postMessage(
						{ type: "action-request", v: 1, requestId, action },
						target,
					);
				});
			},
		[],
	);

	return (
		<TabContext.Provider
			value={{
				actorId,
				queryClient,
				sendAction,
				features,
				rivetkitVersion,
				inspectorProtocolVersion,
				isInitialized,
			}}
		>
			{children}
		</TabContext.Provider>
	);
}
