import {
	useDialog as baseUseDialog,
	createDialogHook,
} from "@/components/actors";

export const useDialog = {
	...baseUseDialog,
	CreateNamespace: createDialogHook(
		() => import("@/app/dialogs/create-namespace-dialog"),
	),
	CreateProject: createDialogHook(
		() => import("@/app/dialogs/create-project-dialog"),
	),
	ConnectVercel: createDialogHook(
		() => import("@/app/dialogs/connect-vercel-frame"),
	),
	ConnectRailway: createDialogHook(
		() => import("@/app/dialogs/connect-railway-frame"),
	),
	ProvideEngineCredentials: createDialogHook(
		() => import("@/app/dialogs/provide-engine-credentials-dialog"),
	),
};
