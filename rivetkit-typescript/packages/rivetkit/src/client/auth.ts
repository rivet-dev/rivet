import type { Rivet } from "@rivetkit/engine-api-full";

/** A namespace-scoped Engine grant. The caller must provide every grant explicitly. */
export type TokenGrant = Rivet.AuthTokenGrant;

/** Options for `client.auth.issueToken()`. `expiresIn` is in seconds. */
export type IssueTokenOptions = Pick<
	Rivet.AuthTokenCreateRequest,
	"subject" | "grants"
> & {
	expiresIn?: Rivet.AuthTokenCreateRequest["duration"];
};

/** Token timestamps are Unix milliseconds. */
export interface IssuedToken {
	token: Rivet.AuthTokenCreateResponse["token"];
	issuedAt: Rivet.AuthTokenCreateResponse["issuedTs"];
	expiresAt: Rivet.AuthTokenCreateResponse["expiresTs"];
}

/** Resources that can be scoped to a single actor ID. */
export type ActorTokenResource = Extract<
	Rivet.AuthTokenGrant["resource"],
	"actor_gateway" | "actor" | "actor_kv"
>;

export type ActorTokenPermissions = Partial<
	Record<ActorTokenResource, Rivet.AuthTokenGrant["operations"]>
>;

/** Options for `actor.issueToken()`. Omit permissions for gateway read access only. */
export type ActorIssueTokenOptions = Pick<
	IssueTokenOptions,
	"subject" | "expiresIn"
> & {
	permissions?: ActorTokenPermissions;
};
