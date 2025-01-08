import {
	Dialog,
	DialogContent,
	type DialogProps,
	DialogTitle,
	VisuallyHidden,
	cn,
} from "@rivet-gg/components";
import {
	type ComponentProps,
	type ComponentType,
	lazy,
	useCallback,
	useMemo,
	useState,
} from "react";

export interface DialogContentProps {
	onClose?: () => void;
}

interface DialogConfig {
	autoFocus?: boolean;
	size?: "md" | "lg";
}

export const createDialogHook = <
	// biome-ignore lint/suspicious/noExplicitAny: we don't know the type of the component, so we use any
	Component extends Promise<{ default: ComponentType<any> }>,
>(
	component: Component,
	opts: DialogConfig = { size: "md" },
) => {
	const DialogImpl = ({
		dialogProps,
		...props
	}: ComponentProps<Awaited<Component>["default"]> & {
		dialogProps?: DialogProps;
	}) => {
		// biome-ignore lint/correctness/useExhaustiveDependencies: component here is a static value, won't change over time
		const Content = useMemo(() => lazy(() => component), []);

		return (
			<Dialog {...dialogProps}>
				<DialogContent
					className={cn({
						"max-w-2xl": opts.size === "lg",
						"max-w-xl": opts.size === "md",
					})}
					onOpenAutoFocus={(e) => {
						if (opts.autoFocus === false) {
							return e.preventDefault();
						}
					}}
				>
					<VisuallyHidden>
						<DialogTitle>Dynamic title</DialogTitle>
					</VisuallyHidden>
					<Content
						{...props}
						onClose={() => dialogProps?.onOpenChange?.(false)}
					/>
				</DialogContent>
			</Dialog>
		);
	};

	const useHook = (props: ComponentProps<Awaited<Component>["default"]>) => {
		const [isOpen, setIsOpen] = useState(() => false);

		const close = useCallback(() => {
			setIsOpen(false);
		}, []);

		const open = useCallback(() => {
			setIsOpen(true);
		}, []);

		const handleOpenChange = useCallback((open: boolean) => {
			setIsOpen(open);
		}, []);

		return {
			open,
			close,
			dialog: (
				<DialogImpl
					{...props}
					dialogProps={{
						open: isOpen,
						onOpenChange: handleOpenChange,
					}}
				/>
			),
		};
	};

	useHook.Dialog = DialogImpl;

	return useHook;
};

export const createDataDialogHook = <
	const DataPropKeys extends string[],
	// biome-ignore lint/suspicious/noExplicitAny: we don't know the type of the component, so we use any
	Component extends Promise<{ default: ComponentType<any> }>,
>(
	keys: DataPropKeys,
	component: Component,
	opts: DialogConfig = {},
) => {
	return (
		props: Omit<
			ComponentProps<Awaited<Component>["default"]>,
			DataPropKeys[number]
		>,
	) => {
		const [isOpen, setIsOpen] = useState(false);
		const [data, setData] =
			useState<
				Pick<
					ComponentProps<Awaited<Component>["default"]>,
					DataPropKeys[number]
				>
			>();

		const close = useCallback(() => {
			setIsOpen(false);
		}, []);

		const open = useCallback(
			(
				data: Pick<
					ComponentProps<Awaited<Component>["default"]>,
					DataPropKeys[number]
				>,
			) => {
				setIsOpen(true);
				setData(data);
			},
			[],
		);

		// biome-ignore lint/correctness/useExhaustiveDependencies: component here is a static value, won't change over time
		const Content = useMemo(() => lazy(() => component), []);

		return {
			open,
			dialog: (
				<Dialog open={isOpen} onOpenChange={setIsOpen}>
					<DialogContent
						onOpenAutoFocus={(e) => {
							if (opts.autoFocus === false) {
								return e.preventDefault();
							}
						}}
					>
						<Content {...props} {...data} onClose={close} />
					</DialogContent>
				</Dialog>
			),
		};
	};
};

export function useDialog() {}

useDialog.GenerateEnvironmentPublicToken = createDialogHook(
	import(
		"@/domains/project/components/dialogs/environment-generate-public-token-dialog"
	),
	{
		autoFocus: false,
	},
);

useDialog.GenerateProjectCloudToken = createDialogHook(
	import(
		"@/domains/project/components/dialogs/project-generate-cloud-token-dialog"
	),
	{
		autoFocus: false,
	},
);
useDialog.GenerateProjectEnvServiceToken = createDialogHook(
	import(
		"@/domains/project/components/dialogs/environment-generate-service-token-dialog"
	),
	{
		autoFocus: false,
	},
);

useDialog.CreateGroupProject = createDialogHook(
	import("@/domains/project/components/dialogs/group-create-project-dialog"),
);

useDialog.CreateProject = createDialogHook(
	import("@/domains/project/components/dialogs/create-project-dialog"),
);

useDialog.ManageCdnAuthUsers = createDialogHook(
	import("@/domains/project/components/dialogs/cdn-manage-auth-users-dialog"),
);

useDialog.CreateEnvironment = createDialogHook(
	import("@/domains/project/components/dialogs/create-environment-dialog"),
);

useDialog.ManageCdnCustomDomains = createDialogHook(
	import(
		"@/domains/project/components/dialogs/cdn-manage-custom-domains-dialog"
	),
);

useDialog.DeployEnvironmentVersion = createDialogHook(
	import(
		"@/domains/project/components/dialogs/deploy-environment-version-dialog"
	),
);

useDialog.ConfirmBillingPlan = createDataDialogHook(
	["plan"],
	import("@/domains/project/components/dialogs/confirm-billing-plan-dialog"),
);

useDialog.CreateGroupInvite = createDialogHook(
	import("@/domains/group/components/dialogs/create-group-invite-dialog"),
);

useDialog.ConfirmTransferOwnership = createDataDialogHook(
	["identityId"],
	import(
		"@/domains/group/components/dialogs/confirm-transfer-ownership-dialog"
	),
);

useDialog.ConfirmMemberKick = createDataDialogHook(
	["identityId"],
	import("@/domains/group/components/dialogs/confirm-member-kick-dialog"),
);

useDialog.ConfirmMemberBan = createDataDialogHook(
	["identityId"],
	import("@/domains/group/components/dialogs/confirm-member-ban-dialog"),
);

useDialog.CreateGroup = createDialogHook(
	import("@/domains/group/components/dialogs/create-group-dialog"),
);

useDialog.ConfirmAccountDeletion = createDialogHook(
	import("@/domains/user/components/dialogs/confirm-account-deletion-dialog"),
);

useDialog.Feedback = createDialogHook(
	import("@//components/dialogs/feedback-dialog"),
);

useDialog.Secret = createDialogHook(
	import("@//components/dialogs/secret-dialog"),
);

useDialog.ConfirmOuterbaseConnection = createDialogHook(
	import(
		"@/domains/project/components/dialogs/confirm-outerbase-connection-dialog"
	),
);

useDialog.ConfirmLeaveGroup = createDialogHook(
	import("@/domains/group/components/dialogs/confirm-leave-group-dialog"),
);

useDialog.EditBuildTags = createDialogHook(
	import("@/domains/project/components/dialogs/edit-build-tags-dialog"),
);
