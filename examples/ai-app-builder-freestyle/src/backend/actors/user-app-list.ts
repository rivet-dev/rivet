import { actor } from "rivetkit";
import type { registry } from "../actors";
import { freestyle } from "../freestyle";

/**
 * UserAppList actor - stores the list of all user apps (for browsing)
 * Single instance that tracks all app IDs and handles app creation
 */
export const userAppList = actor({
	state: {
		appIds: [] as string[],
	},

	actions: {
		/**
		 * Create a new app - creates git repo, adds the app ID to the list, and initializes the userApp actor
		 */
		createApp: async (
			c,
			{
				appId,
				name,
				templateUrl,
				templateId,
			}: {
				appId: string;
				name: string;
				templateUrl: string;
				templateId: string;
			},
		) => {
			const client = c.client<typeof registry>();

			// Create git repository
			const repo = await freestyle.createGitRepository({
				name,
				public: true,
				// source: {
				// 	url: "https://github.com/freestyle-sh/freestyle-next",
				// 	type: "git",
				// },

				// import:
				source: {
					type: "git",
					url: templateUrl,
				},
			});
			const gitRepo = repo.repoId;

			// Add the app ID to the list
			if (!c.state.appIds.includes(appId)) {
				c.state.appIds.push(appId);
			}

			// Create the userApp actor with input (the actor is initialized with this data)
			// Call getInfo to ensure the actor is fully created before returning
			const userAppHandle = client.userApp.getOrCreate([appId], {
				createWithInput: {
					name,
					description: "No description",
					gitRepo,
					templateId,
				},
			});
			// Wait for the actor to be ready by calling an action on it
			await userAppHandle.getInfo();

			return { appId, gitRepo };
		},

		getAppIds: (c) => c.state.appIds,
	},
});
