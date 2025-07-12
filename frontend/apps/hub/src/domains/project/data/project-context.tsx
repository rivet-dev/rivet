import { useAuth } from "@/domains/auth/contexts/auth";
import { ls } from "@/lib/ls";
import type { Rivet } from "@rivet-gg/api-full";
import { useSuspenseQuery } from "@tanstack/react-query";
import { type ReactNode, createContext, useContext, useEffect } from "react";
import { projectByIdQueryOptions } from "../queries";

export const ProjectContext = createContext<Rivet.game.GameSummary | undefined>(
	undefined,
);

export function ProjectContextProvider({
	children,
	projectNameId,
}: {
	children: ReactNode;
	projectNameId: string;
}) {
	const auth = useAuth();
	const { data: project } = useSuspenseQuery(
		projectByIdQueryOptions(projectNameId),
	);

	useEffect(() => {
		ls.recentTeam.set(auth, project.developer.groupId);
	}, [auth, project.developer.groupId]);

	return (
		<ProjectContext.Provider value={project}>
			{children}
		</ProjectContext.Provider>
	);
}

function useProjectContext() {
	const context = useContext(ProjectContext);
	if (!context) {
		throw new Error(
			"useProjectContext must be used within a ProjectContextProvider",
		);
	}
	return context;
}

export function useProject() {
	return useProjectContext();
}
