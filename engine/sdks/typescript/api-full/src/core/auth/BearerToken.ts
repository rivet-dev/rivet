export type BearerToken = string;

const BEARER_AUTH_HEADER_PREFIX = /^Bearer /i;

export const BearerToken = {
    toAuthorizationHeader: (token: BearerToken | undefined): string | undefined => {
        if (token == null) {
            return undefined;
        }
        return `Bearer ${token}`;
    },
    fromAuthorizationHeader: (header: string): BearerToken => {
        return header.replace(BEARER_AUTH_HEADER_PREFIX, "").trim() as BearerToken;
    },
};
