import {
	getTrustedShellOrigin,
	type InitMessage,
	type InspectorTabDescriptor,
	PROTOCOL_VERSION,
	type SetActiveTabMessage,
	shellToIframeMessageSchema,
} from "./bridge";

type InitListener = (init: InitMessage) => void;
type SetActiveTabListener = (msg: SetActiveTabMessage) => void;

// Subscribes to the shell's postMessage stream and exposes:
//   • onInit(cb) — fired on every `init` (including token-refresh-driven ones)
//   • onSetActiveTab(cb) — fired when the shell tells us to switch tabs
//   • sendReady() — called once the React app has mounted
//   • sendTabsAvailable(tabs) — called after init to populate the dashboard
//     tab strip with this actor's supported tabs
//   • requestTokenRefresh() — called on WS auth failure
//
// Origin enforcement: messages are accepted only when event.origin matches the
// `shellOrigin` URL param (or window.location.origin for same-origin shells).
// Other origins are silently dropped so a hostile page embedded in another
// iframe cannot spoof an `init`.
export class BridgeClient {
	private readonly trustedOrigin: string;
	private readonly initListeners = new Set<InitListener>();
	private readonly setActiveTabListeners = new Set<SetActiveTabListener>();
	private latestInit: InitMessage | null = null;
	private bound = false;
	private readyPending = true;

	constructor() {
		this.trustedOrigin = getTrustedShellOrigin() ?? window.location.origin;
	}

	get shellOrigin(): string {
		return this.trustedOrigin;
	}

	start() {
		if (this.bound) return;
		this.bound = true;
		window.addEventListener("message", this.handleMessage);
	}

	stop() {
		if (!this.bound) return;
		this.bound = false;
		window.removeEventListener("message", this.handleMessage);
		this.initListeners.clear();
		this.setActiveTabListeners.clear();
	}

	onInit(cb: InitListener): () => void {
		this.initListeners.add(cb);
		if (this.latestInit) cb(this.latestInit);
		return () => {
			this.initListeners.delete(cb);
		};
	}

	onSetActiveTab(cb: SetActiveTabListener): () => void {
		this.setActiveTabListeners.add(cb);
		return () => {
			this.setActiveTabListeners.delete(cb);
		};
	}

	sendReady() {
		if (!this.readyPending) return;
		this.readyPending = false;
		this.postToShell({ type: "ready", v: PROTOCOL_VERSION });
	}

	sendTabsAvailable(tabs: InspectorTabDescriptor[]) {
		this.postToShell({
			type: "tabs-available",
			v: PROTOCOL_VERSION,
			tabs,
		});
	}

	requestTokenRefresh() {
		this.postToShell({ type: "token-refresh-needed", v: PROTOCOL_VERSION });
	}

	private handleMessage = (event: MessageEvent) => {
		if (event.origin !== this.trustedOrigin) return;
		const parsed = shellToIframeMessageSchema.safeParse(event.data);
		if (!parsed.success) return;
		const msg = parsed.data;
		if (msg.type === "init") {
			this.latestInit = msg;
			for (const cb of this.initListeners) cb(msg);
		} else if (msg.type === "set-active-tab") {
			for (const cb of this.setActiveTabListeners) cb(msg);
		}
	};

	private postToShell(payload: unknown) {
		// targetOrigin is the trusted shell origin; the browser refuses to
		// deliver if the parent isn't actually at that origin, so credentials-
		// adjacent messages (token-refresh-needed) never leak.
		window.parent.postMessage(payload, this.trustedOrigin);
	}
}
