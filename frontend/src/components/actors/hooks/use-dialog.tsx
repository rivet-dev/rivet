"use client";
import { VisuallyHidden } from "@radix-ui/react-visually-hidden";
import {
	type ComponentProps,
	type ComponentType,
	lazy,
	Suspense,
	useCallback,
	useMemo,
	useState,
} from "react";
import { Skeleton } from "@/components/ui/skeleton";
import {
	Dialog,
	DialogContent,
	type DialogProps,
	DialogTitle,
} from "../../ui/dialog";

export interface DialogContentProps {
	onClose?: () => void;
}

interface DialogConfig {
	autoFocus?: boolean;
}

export const createDialogHook = <
	// biome-ignore lint/suspicious/noExplicitAny: we don't know the type of the component, so we use any
	Component extends Promise<{ default: ComponentType<any> }>,
>(
	component: Component,
	opts: DialogConfig = {},
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
					onOpenAutoFocus={(e) => {
						if (opts.autoFocus === false) {
							return e.preventDefault();
						}
					}}
				>
					<Suspense
						fallback={
							<div className="flex flex-col gap-4">
								<VisuallyHidden>
									<DialogTitle>Loading...</DialogTitle>
								</VisuallyHidden>
								<div className="flex flex-col">
									<Skeleton className="w-1/4 h-5" />
									<Skeleton className="w-3/4 h-5 mt-2" />
								</div>

								<div className="flex flex-col gap-2">
									<Skeleton className="w-1/3 h-5" />
									<Skeleton className="w-full h-10" />
								</div>
								<div className="flex flex-col gap-2">
									<Skeleton className="w-1/3 h-5" />
									<Skeleton className="w-full h-10" />
								</div>
								<div className="flex flex-col gap-2">
									<Skeleton className="w-1/3 h-5" />
									<Skeleton className="w-full h-10" />
								</div>
								<div className="flex flex-col gap-2">
									<Skeleton className="w-1/3 h-5" />
									<Skeleton className="w-full h-10" />
								</div>
								<div className="flex flex-col gap-2">
									<Skeleton className="w-1/3 h-5" />
									<Skeleton className="w-full h-10" />
								</div>
							</div>
						}
					>
						<Content
							{...props}
							onClose={() => dialogProps?.onOpenChange?.(false)}
						/>
					</Suspense>
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
	_: DataPropKeys,
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

useDialog.GoToActor = createDialogHook(import("../dialogs/go-to-actor-dialog"));

useDialog.Feedback = createDialogHook(import("../../dialogs/feedback-dialog"));
useDialog.CreateActor = createDialogHook(
	import("../dialogs/create-actor-dialog"),
);
