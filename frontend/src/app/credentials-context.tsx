import { createContext, useContext } from "react";

export type InspectorCredentials = {
	url: string;
	token: string;
};

export const InspectorCredentialsContext = createContext<{
	credentials: InspectorCredentials | null;
	setCredentials: (creds: InspectorCredentials | null) => void;
}>({ credentials: null, setCredentials: () => {} });

export const useInspectorCredentials = () => {
	const ctx = useContext(InspectorCredentialsContext);
	return ctx;
};

export const InspectorCredentialsProvider =
	InspectorCredentialsContext.Provider;
