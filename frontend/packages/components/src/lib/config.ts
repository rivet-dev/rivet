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

const getApiEndpoint = (apiEndpoint: string) => {
	// __SAME__ is used in Docker builds to serve API from the same origin as the frontend
	if (apiEndpoint === "__SAME__") {
		return location.origin;
	}
	return apiEndpoint;
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
