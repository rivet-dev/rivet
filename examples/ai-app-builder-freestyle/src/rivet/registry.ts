import { actor, setup } from "rivetkit";
import type { UIMessage } from "ai";

// Types
export interface AppInfo {
	id: string;
	name: string;
	description: string;
	gitRepo: string;
	createdAt: number;
	baseId: string;
	previewDomain: string | null;
	// Freestyle identity for this app
	freestyleIdentity: string | null;
	freestyleAccessToken: string | null;
	freestyleAccessTokenId: string | null;
}

export interface AppDeployment {
	deploymentId: string;
	commit: string;
	createdAt: number;
}

/**
 * AppStore actor - stores data for a single app
 * Each app gets its own actor instance keyed by app ID
 */
export const appStore = actor({
	state: {
		info: null as AppInfo | null,
		messages: [] as UIMessage[],
		deployments: [] as AppDeployment[],
	},

	actions: {
		// App info operations
		createApp: (
			c,
			info: Omit<AppInfo, "id" | "createdAt">
		) => {
			const appInfo: AppInfo = {
				...info,
				id: c.key[0] as string,
				createdAt: Date.now(),
			};
			c.state.info = appInfo;
			return appInfo;
		},

		getInfo: (c) => c.state.info,

		updateInfo: (c, updates: Partial<Omit<AppInfo, "id" | "createdAt">>) => {
			if (c.state.info) {
				c.state.info = { ...c.state.info, ...updates };
			}
			return c.state.info;
		},

		// Message operations
		addMessage: (c, message: UIMessage) => {
			c.state.messages.push(message);
			c.broadcast("newMessage", message);
			return message;
		},

		getMessages: (c) => c.state.messages,

		clearMessages: (c) => {
			c.state.messages = [];
			return { success: true };
		},

		// Deployment operations
		addDeployment: (c, deployment: Omit<AppDeployment, "createdAt">) => {
			const appDeployment: AppDeployment = {
				...deployment,
				createdAt: Date.now(),
			};
			c.state.deployments.push(appDeployment);
			return appDeployment;
		},

		getDeployments: (c) => c.state.deployments,

		// Get all data
		getAll: (c) => ({
			info: c.state.info,
			messages: c.state.messages,
			deployments: c.state.deployments,
		}),

		// Delete app (clears the actor state)
		deleteApp: (c) => {
			c.state.info = null;
			c.state.messages = [];
			c.state.deployments = [];
			return { success: true };
		},
	},
});

/**
 * AppList actor - stores the list of all apps (for browsing)
 * Single instance that tracks all app IDs
 */
export const appList = actor({
	state: {
		appIds: [] as string[],
	},

	actions: {
		addApp: (c, appId: string) => {
			if (!c.state.appIds.includes(appId)) {
				c.state.appIds.push(appId);
			}
			return c.state.appIds;
		},

		removeApp: (c, appId: string) => {
			c.state.appIds = c.state.appIds.filter((id) => id !== appId);
			return c.state.appIds;
		},

		getAppIds: (c) => c.state.appIds,
	},
});

/**
 * StreamState actor - manages stream state for an app
 * Replaces Redis for stream state management
 */
export const streamState = actor({
	state: {
		status: null as string | null,
		lastUpdate: 0,
	},

	actions: {
		setRunning: (c) => {
			c.state.status = "running";
			c.state.lastUpdate = Date.now();
			return { status: c.state.status };
		},

		getStatus: (c) => {
			// Auto-expire after 15 seconds
			if (c.state.status && Date.now() - c.state.lastUpdate > 15000) {
				c.state.status = null;
			}
			return c.state.status;
		},

		clear: (c) => {
			c.state.status = null;
			return { success: true };
		},

		// Refresh the keep-alive
		keepAlive: (c) => {
			if (c.state.status === "running") {
				c.state.lastUpdate = Date.now();
			}
			return { success: true };
		},

		// Broadcast abort event
		abort: (c) => {
			c.broadcast("abort");
			c.state.status = null;
			return { success: true };
		},
	},
});

// Register actors for use
export const registry = setup({
	use: { appStore, appList, streamState },
});
