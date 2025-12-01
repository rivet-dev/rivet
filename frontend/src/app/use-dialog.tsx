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
	ConnectQuickVercel: createDialogHook(
		() => import("@/app/dialogs/connect-quick-vercel-frame"),
	),
	ConnectRailway: createDialogHook(
		() => import("@/app/dialogs/connect-railway-frame"),
	),
	ConnectQuickRailway: createDialogHook(
		() => import("@/app/dialogs/connect-quick-railway-frame"),
	),
	ConnectManual: createDialogHook(
		() => import("@/app/dialogs/connect-manual-frame"),
	),
	ConnectAws: createDialogHook(
		() => import("@/app/dialogs/connect-aws-frame"),
	),
	ConnectGcp: createDialogHook(
		() => import("@/app/dialogs/connect-gcp-frame"),
	),
	ConnectHetzner: createDialogHook(
		() => import("@/app/dialogs/connect-hetzner-frame"),
	),
	EditProviderConfig: createDialogHook(
		() => import("@/app/dialogs/edit-runner-config"),
	),
	DeleteConfig: createDialogHook(
		() => import("@/app/dialogs/confirm-delete-config-frame"),
	),
	Billing: createDialogHook(() => import("@/app/dialogs/billing-frame")),
	ProvideEngineCredentials: createDialogHook(
		() => import("@/app/dialogs/provide-engine-credentials-frame"),
	),
	Tokens: createDialogHook(() => import("@/app/dialogs/tokens-frame")),
	StartWithTemplate: createDialogHook(
		() => import("@/app/dialogs/start-with-template-frame"),
	),
};
