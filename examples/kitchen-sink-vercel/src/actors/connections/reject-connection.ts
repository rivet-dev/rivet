import { actor, UserError } from "rivetkit";

export const rejectConnectionActor = actor({
	onBeforeConnect: async (_c, params: { reject?: boolean }) => {
		if (params?.reject) {
			await new Promise((resolve) => setTimeout(resolve, 500));
			throw new UserError("Rejected connection", {
				code: "rejected",
			});
		}
	},
	actions: {
		ping: () => "pong",
	},
});
