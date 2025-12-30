// import crypto from "node:crypto";
import { createMiddleware } from "hono/factory";
import type { ManagerDriver } from "@/driver-helpers/mod";
import { inspectorLogger } from "./log";
import { BaseConfig } from "@/registry/config/base";

export function compareSecrets(providedSecret: string, validSecret: string) {
	// Early length check to avoid unnecessary processing
	if (providedSecret.length !== validSecret.length) {
		return false;
	}

	const encoder = new TextEncoder();

	const a = encoder.encode(providedSecret);
	const b = encoder.encode(validSecret);

	if (a.byteLength !== b.byteLength) {
		return false;
	}

	// TODO:
	// // Perform timing-safe comparison
	// if (!crypto.timingSafeEqual(a, b)) {
	// 	return false;
	// }
	return true;
}

export const secureInspector = (config: BaseConfig) =>
	createMiddleware(async (c, next) => {
		const userToken = c.req.header("Authorization")?.replace("Bearer ", "");
		if (!userToken) {
			return c.text("Unauthorized", 401);
		}

		const inspectorToken = config.inspector.token();
		if (!inspectorToken) {
			return c.text("Unauthorized", 401);
		}

		const isValid = compareSecrets(userToken, inspectorToken);
		if (!isValid) {
			return c.text("Unauthorized", 401);
		}
		await next();
	});

export function getInspectorUrl(config: BaseConfig): string | undefined {
	if (!config.inspector.enabled) {
		return undefined;
	}

	const accessToken = config.inspector.token();
	if (!accessToken) {
		inspectorLogger().warn(
			"Inspector Token is not set, but Inspector is enabled. Please set it in the run configuration `inspector.token` or via `RIVETKIT_INSPECTOR_TOKEN` environment variable. Inspector will not be accessible.",
		);
		return undefined;
	}

	const url = new URL("https://inspect.rivet.dev");
	url.searchParams.set("t", accessToken);

	// Only override endpoint if using non-default port or custom endpoint is set
	const endpoint =
		config.inspector.defaultEndpoint ??
		(config.managerPort !== 6420
			? `http://localhost:${config.managerPort}`
			: undefined);
	if (endpoint) {
		url.searchParams.set("u", endpoint);
	}

	return url.href;
}

export const isInspectorEnabled = (
	config: BaseConfig,
	// TODO(kacper): Remove context in favor of using the gateway, so only context is the actor
	context: "actor" | "manager",
) => {
	if (typeof config.inspector.enabled === "boolean") {
		return config.inspector.enabled;
	} else if (typeof config.inspector.enabled === "object") {
		return config.inspector.enabled[context];
	}
	return false;
};

export const configureInspectorAccessToken = (
	config: BaseConfig,
	managerDriver: ManagerDriver,
) => {
	if (!config.inspector.token()) {
		const token = managerDriver.getOrCreateInspectorAccessToken();
		config.inspector.token = () => token;
	}
};
