import { Base64 } from "js-base64";

export interface BasicAuth {
    username: string;
    password: string;
}

const BASIC_AUTH_HEADER_PREFIX = /^Basic /i;

export const BasicAuth = {
    toAuthorizationHeader: (basicAuth: BasicAuth | undefined): string | undefined => {
        if (basicAuth == null) {
            return undefined;
        }
        const token = Base64.encode(`${basicAuth.username}:${basicAuth.password}`);
        return `Basic ${token}`;
    },
    fromAuthorizationHeader: (header: string): BasicAuth => {
        const credentials = header.replace(BASIC_AUTH_HEADER_PREFIX, "");
        const decoded = Base64.decode(credentials);
        const [username, password] = decoded.split(":", 2);

        if (username == null || password == null) {
            throw new Error("Invalid basic auth");
        }
        return {
            username,
            password,
        };
    },
};
