import { create } from "zustand";
import { persist } from "zustand/middleware";
import { cloudApi } from "@/lib/cloud-api/client";
import type {
	CloudBootstrapResponse,
	CloudNamespace,
	CloudNamespaceToken,
	CloudProject,
} from "@/lib/cloud-api/types";

interface AutofillState {
	// Data
	user: CloudBootstrapResponse["user"] | null;
	projects: CloudProject[];
	namespaces: CloudNamespace[];
	selectedProjectId: string | null;
	selectedNamespaceId: string | null;
	tokenData: CloudNamespaceToken | null;

	// Loading states
	isLoading: boolean;
	isLoadingNamespaces: boolean;
	isLoadingToken: boolean;

	// Actions
	initialize: () => Promise<void>;
	selectProject: (projectId: string) => Promise<void>;
	selectNamespace: (namespaceId: string) => Promise<void>;
	getTemplateVariables: () => Record<string, string>;
}

export const useAutofillStore = create<AutofillState>()(
	persist(
		(set, get) => ({
			// Initial state
			user: null,
			projects: [],
			namespaces: [],
			selectedProjectId: null,
			selectedNamespaceId: null,
			tokenData: null,
			isLoading: false,
			isLoadingNamespaces: false,
			isLoadingToken: false,

			// Initialize: fetch bootstrap data
			initialize: async () => {
				set({ isLoading: true });
				try {
					const data = await cloudApi.bootstrap();
					set({
						user: data.user,
						projects: data.projects,
						isLoading: false,
					});

					// Auto-select first project if available and no selection exists
					const state = get();
					if (data.projects.length > 0 && !state.selectedProjectId) {
						await get().selectProject(data.projects[0].id);
					}
				} catch (error) {
					console.error("Failed to initialize autofill:", error);
					set({ isLoading: false });
				}
			},

			// Select a project and load its namespaces
			selectProject: async (projectId: string) => {
				set({
					selectedProjectId: projectId,
					isLoadingNamespaces: true,
					selectedNamespaceId: null,
					tokenData: null,
				});

				try {
					const namespaces = await cloudApi.getNamespaces(projectId);
					set({
						namespaces,
						isLoadingNamespaces: false,
					});

					// Auto-select first namespace
					if (namespaces.length > 0) {
						await get().selectNamespace(namespaces[0].id);
					}
				} catch (error) {
					console.error("Failed to load namespaces:", error);
					set({ isLoadingNamespaces: false });
				}
			},

			// Select a namespace and fetch its token
			selectNamespace: async (namespaceId: string) => {
				const { selectedProjectId } = get();
				if (!selectedProjectId) return;

				set({
					selectedNamespaceId: namespaceId,
					isLoadingToken: true,
				});

				try {
					const tokenData = await cloudApi.getNamespaceToken(
						selectedProjectId,
						namespaceId,
					);
					set({
						tokenData,
						isLoadingToken: false,
					});
				} catch (error) {
					console.error("Failed to load token:", error);
					set({ isLoadingToken: false });
				}
			},

			// Get all template variables for replacement
			getTemplateVariables: () => {
				const state = get();
				const project = state.projects.find(
					(p) => p.id === state.selectedProjectId,
				);
				const namespace = state.namespaces.find(
					(n) => n.id === state.selectedNamespaceId,
				);

				return {
					"project.slug": project?.slug || "",
					"project.name": project?.name || "",
					"organization.slug": project?.organizationSlug || "",
					"namespace.slug": namespace?.slug || "",
					"namespace.name": namespace?.name || "",
					"namespace.token": state.tokenData?.token || "",
					"engine.url": state.tokenData?.engineUrl || "",
				};
			},
		}),
		{
			name: "rivet-autofill-storage",
			partialize: (state) => ({
				selectedProjectId: state.selectedProjectId,
				selectedNamespaceId: state.selectedNamespaceId,
			}),
		},
	),
);
