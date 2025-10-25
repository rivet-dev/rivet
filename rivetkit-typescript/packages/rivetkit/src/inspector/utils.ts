// import crypto from "node:crypto";
import { createMiddleware } from "hono/factory";
import type { ManagerDriver } from "@/driver-helpers/mod";
import type { RunConfig } from "@/mod";
import type { RunnerConfigInput } from "@/registry/run-config";
import { inspectorLogger } from "./log";

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

export const secureInspector = (runConfig: RunConfig) =>
	createMiddleware(async (c, next) => {
		const userToken = c.req.header("Authorization")?.replace("Bearer ", "");
		if (!userToken) {
			return c.text("Unauthorized", 401);
		}

		const inspectorToken = runConfig.inspector.token?.();
		if (!inspectorToken) {
			return c.text("Unauthorized", 401);
		}

		const isValid = compareSecrets(userToken, inspectorToken);

		if (!isValid) {
			return c.text("Unauthorized", 401);
		}
		await next();
	});

export function getInspectorUrl(runConfig: RunnerConfigInput | undefined) {
	if (!runConfig?.inspector?.enabled) {
		return "disabled";
	}

	const accessToken = runConfig?.inspector?.token?.();

	if (!accessToken) {
		inspectorLogger().warn(
			"Inspector Token is not set, but Inspector is enabled. Please set it in the run configuration `inspector.token` or via `RIVETKIT_INSPECTOR_TOKEN` environment variable. Inspector will not be accessible.",
		);
		return "disabled";
	}

	const url = new URL("https://inspect.rivet.dev");

	url.searchParams.set("t", accessToken);

	const overrideDefaultEndpoint =
		runConfig?.inspector?.defaultEndpoint ??
		runConfig.overrideServerAddress;
	if (overrideDefaultEndpoint) {
		url.searchParams.set("u", overrideDefaultEndpoint);
	}

	return url.href;
}

export const isInspectorEnabled = (
	runConfig: RunConfig,
	context: "actor" | "manager",
) => {
	if (typeof runConfig.inspector?.enabled === "boolean") {
		return runConfig.inspector.enabled;
	} else if (typeof runConfig.inspector?.enabled === "object") {
		return runConfig.inspector.enabled[context];
	}
	return false;
};

export const configureInspectorAccessToken = (
	runConfig: RunConfig,
	managerDriver: ManagerDriver,
) => {
	if (!runConfig.inspector?.token()) {
		const token = managerDriver.getOrCreateInspectorAccessToken();
		runConfig.inspector.token = () => token;
	}
};
