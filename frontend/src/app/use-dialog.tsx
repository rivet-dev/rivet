import {
	useDialog as baseUseDialog,
	createDialogHook,
} from "@/components/actors";

export const useDialog = {
	...baseUseDialog,
	CreateNamespace: createDialogHook(
		import("@/app/dialogs/create-namespace-dialog"),
	),
	ProvideEngineCredentials: createDialogHook(
		import("@/app/dialogs/provide-engine-credentials-dialog"),
	),
};
