import { useSuspenseQuery } from "@tanstack/react-query";
import { environmentByIdQueryOptions } from "../queries";
import { type ReactNode, createContext, useContext } from "react";
import type { Rivet } from "@rivet-gg/api";
import { useProject } from "./project-context";

export const EnvironmentContext = createContext<
	Rivet.cloud.NamespaceSummary | undefined
>(undefined);

export function EnvironmentContextProvider({
	children,
	environmentNameId,
}: {
	children: ReactNode;
	environmentNameId: string;
}) {
	const project = useProject();
	const { data: environment } = useSuspenseQuery(
		environmentByIdQueryOptions({
			projectId: project.gameId,
			environmentNameId,
		}),
	);

	return (
		<EnvironmentContext.Provider value={environment}>
			{children}
		</EnvironmentContext.Provider>
	);
}

function useEnvironmentContext() {
	const context = useContext(EnvironmentContext);
	if (!context) {
		throw new Error(
			"useEnvironmentContext must be used within a EnvironmentContextProvider",
		);
	}
	return context;
}

export function useEnvironment() {
	return useEnvironmentContext();
}
