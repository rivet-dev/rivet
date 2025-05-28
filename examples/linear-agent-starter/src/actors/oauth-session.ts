import { actor } from "actor-core";

interface OAuthSessionInput {
	oauthState: string;
}

interface OAuthSessoinState {
	oauthState: string;
	accessToken?: string;
}

export interface OAuthExpectedState {
	sessionId: string;
	nonce: string;
}

export const oauthSession = actor({
	createState: (c, opts) =>
		({
			oauthState: (opts.input as OAuthSessionInput).oauthState,
		}) satisfies OAuthSessoinState,
	actions: {
		getOAuthState: (c) => c.state.oauthState,
	},
});
