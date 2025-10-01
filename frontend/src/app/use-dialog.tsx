import { useDialog as baseUseDialog, createDialogHook } from "@/components";

export const useDialog = {
	...baseUseDialog,
	CreateNamespace: createDialogHook(
		() => import("@/app/dialogs/create-namespace-frame"),
	),
	CreateProject: createDialogHook(
		() => import("@/app/dialogs/create-project-frame"),
	),
	ConnectVercel: createDialogHook(
		() => import("@/app/dialogs/connect-vercel-frame"),
	),
	ConnectRailway: createDialogHook(
		() => import("@/app/dialogs/connect-railway-frame"),
	),
	Billing: createDialogHook(() => import("@/app/dialogs/billing-frame")),
	ProvideEngineCredentials: createDialogHook(
		() => import("@/app/dialogs/provide-engine-credentials-frame"),
	),
};
