import { actor } from "rivetkit";

type ConnState = {
	label: string;
};

type ConnParams = {
	label?: string;
	beforeDelayMs?: number;
	createDelayMs?: number;
};

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));
const visibleLabels = (c: {
	conns: Map<unknown, { state?: ConnState }>;
}) =>
	Array.from(c.conns.values()).map((conn) => conn.state?.label ?? null);

export const connPreflightVisibilityActor = actor({
	state: {
		beforeStarted: 0,
		createStarted: 0,
		beforeVisibleLabels: [] as Array<Array<string | null>>,
		createVisibleLabels: [] as Array<Array<string | null>>,
		connectSnapshots: [] as Array<{
			label: string;
			ownVisible: boolean;
			visibleLabels: string[];
		}>,
		disconnectSnapshots: [] as Array<{
			label: string | null;
			otherLabels: Array<string | null>;
		}>,
	},
	onBeforeConnect: async (c, params: ConnParams) => {
		c.state.beforeStarted += 1;
		c.state.beforeVisibleLabels.push(visibleLabels(c));
		if (params?.beforeDelayMs) {
			await sleep(params.beforeDelayMs);
		}
	},
	createConnState: async (c, params: ConnParams): Promise<ConnState> => {
		c.state.createStarted += 1;
		c.state.createVisibleLabels.push(visibleLabels(c));
		if (params?.createDelayMs) {
			await sleep(params.createDelayMs);
		}
		return { label: params?.label ?? "anonymous" };
	},
	onConnect: (c, conn) => {
		c.state.connectSnapshots.push({
			label: conn.state.label,
			ownVisible: c.conns.has(conn.id),
			visibleLabels: Array.from(c.conns.values()).map(
				(other) => other.state.label,
			),
		});
	},
	onDisconnect: (c, conn) => {
		c.state.disconnectSnapshots.push({
			label: conn.state?.label ?? null,
			otherLabels: Array.from(c.conns.values())
				.filter((other) => other !== conn)
				.map((other) => other.state?.label ?? null),
		});
	},
	actions: {
		snapshot: (c) => ({
			beforeStarted: c.state.beforeStarted,
			createStarted: c.state.createStarted,
			beforeVisibleLabels: c.state.beforeVisibleLabels,
			createVisibleLabels: c.state.createVisibleLabels,
			connectSnapshots: c.state.connectSnapshots,
			disconnectSnapshots: c.state.disconnectSnapshots,
			visibleLabels: visibleLabels(c),
		}),
	},
});
