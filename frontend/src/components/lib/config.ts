"use client";
import { createContext, useContext } from "react";

interface Config {
	apiUrl: string;
	assetsUrl: string;
	posthog?: {
		apiHost: string;
		apiKey: string;
	};
	sentry?: {
		dsn: string;
		projectId: string;
		tunnel?: string;
	};
	outerbaseProviderToken: string;
}

export const ConfigContext = createContext<Config>({
	apiUrl: "",
	assetsUrl: "",
	outerbaseProviderToken: "",
});
export const useConfig = () => useContext(ConfigContext);
export const ConfigProvider = ConfigContext.Provider;

export const getApiEndpoint = (apiEndpoint: string) => {
	// __SAME__ is used in Docker builds to serve API from the same origin as the frontend
	if (typeof window !== "undefined" && apiEndpoint === "__SAME__") {
		return window.location.origin;
	}
	return apiEndpoint;
};

/**
 * Returns the PostHog config only when it is actually usable. Unset Vite env
 * vars survive index.html substitution as literal `%VITE_*%` placeholders, so a
 * present `posthog` object is not on its own proof of configuration.
 */
export const getPosthogConfig = (config: Config) => {
	const posthog = config.posthog;
	if (
		!posthog?.apiKey ||
		!posthog.apiHost ||
		posthog.apiKey.startsWith("%")
	) {
		return null;
	}
	return posthog;
};

export const getConfig = (): Config => {
	const el = document.getElementById("RIVET_CONFIG");
	if (!el) {
		throw new Error("Config element not found");
	}

	const parsed = JSON.parse(el.textContent || "");

	return {
		...parsed,
		apiUrl: getApiEndpoint(parsed.apiUrl),
	};
};
