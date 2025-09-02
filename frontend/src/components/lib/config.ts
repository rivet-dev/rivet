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
	if (apiEndpoint === "__AUTO__") {
		if (location.hostname.startsWith("hub.")) {
			// Connect to the corresponding API endpoint
			return `https://${location.hostname.replace("hub.", "api.")}`;
		}
		// Default to staging servers for all other endpoints
		return "https://api.staging2.gameinc.io";
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
