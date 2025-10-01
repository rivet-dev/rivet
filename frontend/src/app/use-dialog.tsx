import {
	useDialog as baseUseDialog,
	createDialogHook,
} from "@/components/actors";

export const useDialog = {
	...baseUseDialog,
	CreateNamespace: createDialogHook(
		import("@/app/dialogs/create-namespace-dialog"),
	),
	CreateProject: createDialogHook(
		import("@/app/dialogs/create-project-dialog"),
	),
	ProvideEngineCredentials: createDialogHook(
		import("@/app/dialogs/provide-engine-credentials-dialog"),
	),
};
