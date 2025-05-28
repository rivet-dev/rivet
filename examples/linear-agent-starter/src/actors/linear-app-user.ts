import { actor } from "actor-core";

interface LinearAppUserState {
	accessToken?: string;
}

export const linearAppUser = actor({
	state: {} as LinearAppUserState,
	actions: {
		setAccessToken: (c, accessToken: string) => {
			c.state.accessToken = accessToken;
		},
		getAccessToken: (c) => {
			return c.state.accessToken;
		},
	},
});
