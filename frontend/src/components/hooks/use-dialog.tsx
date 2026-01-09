"use client";
import { VisuallyHidden } from "@radix-ui/react-visually-hidden";
import { QueryErrorResetBoundary } from "@tanstack/react-query";
import {
	type ComponentProps,
	type ComponentType,
	lazy,
	Suspense,
	useCallback,
	useMemo,
	useState,
} from "react";
import { ErrorBoundary } from "react-error-boundary";
import { Skeleton } from "@/components/ui/skeleton";
import { Button } from "../ui/button";
import {
	Dialog,
	DialogContent,
	type DialogProps,
	DialogTitle,
} from "../ui/dialog";
import {
	Content,
	Footer,
	Header,
	IsInModalContext,
	Title,
} from "./isomorphic-frame";

export interface DialogContentProps {
	onClose?: () => void;
}

interface DialogConfig {
	autoFocus?: boolean;
}

export const createDialogHook = <
	// biome-ignore lint/suspicious/noExplicitAny: we don't know the type of the component, so we use any
	Component extends () => Promise<{ default: ComponentType<any> }>,
>(
	component: Component,
	opts: DialogConfig = {},
) => {
	const DialogImpl = ({
		dialogProps,
		dialogContentProps,
		...props
	}: ComponentProps<Awaited<ReturnType<Component>>["default"]> & {
		dialogProps?: DialogProps;
		dialogContentProps?: ComponentProps<typeof DialogContent>;
	}) => {
		// biome-ignore lint/correctness/useExhaustiveDependencies: component here is a static value, won't change over time
		const Content = useMemo(() => lazy(component), []);

		return (
			<IsInModalContext.Provider value={true}>
				<Dialog
					{...dialogProps}
					onOpenChange={
						props.dismissible === false
							? () => {}
							: dialogProps?.onOpenChange
					}
				>
					<DialogContent
						{...dialogContentProps}
						hideClose={
							props.dismissible === false ||
							dialogContentProps?.hideClose
						}
						disableOutsidePointerEvents={
							props.dismissible === false ||
							dialogContentProps?.disableOutsidePointerEvents
						}
						onOpenAutoFocus={(e) => {
							if (opts.autoFocus === false) {
								return e.preventDefault();
							}
						}}
					>
						<QueryErrorResetBoundary>
							{({ reset }) => (
								<ErrorBoundary
									onReset={reset}
									fallbackRender={({
										error,
										resetErrorBoundary,
									}) => (
										<DialogErrorFallback
											error={error}
											resetError={resetErrorBoundary}
										/>
									)}
								>
									<Suspense
										fallback={
											<div className="flex flex-col gap-4">
												<VisuallyHidden>
													<DialogTitle>
														Loading...
													</DialogTitle>
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
											onClose={() =>
												dialogProps?.onOpenChange?.(
													false,
												)
											}
										/>
									</Suspense>
								</ErrorBoundary>
							)}
						</QueryErrorResetBoundary>
					</DialogContent>
				</Dialog>
			</IsInModalContext.Provider>
		);
	};

	const useHook = (
		props: ComponentProps<Awaited<ReturnType<Component>>["default"]>,
	) => {
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
				<IsInModalContext.Provider value={true}>
					<Dialog open={isOpen} onOpenChange={setIsOpen}>
						<DialogContent
							onOpenAutoFocus={(e) => {
								if (opts.autoFocus === false) {
									return e.preventDefault();
								}
							}}
						>
							<IsInModalContext.Provider value={true}>
								<Content {...props} {...data} onClose={close} />
							</IsInModalContext.Provider>
						</DialogContent>
					</Dialog>
				</IsInModalContext.Provider>
			),
		};
	};
};

export function useDialog() {}

useDialog.GoToActor = createDialogHook(
	() => import("../actors/dialogs/go-to-actor-dialog"),
);

useDialog.Feedback = createDialogHook(
	() => import("../dialogs/feedback-dialog"),
);
useDialog.CreateActor = createDialogHook(
	() => import("../actors/dialogs/create-actor-dialog"),
);

function DialogErrorFallback({
	resetError,
	error,
}: {
	resetError: () => void;
	error: Error;
}) {
	return (
		<>
			<Header>
				<Title>
					{"statusCode" in error && error.statusCode === 404
						? "Resource not found"
						: "body" in error &&
								error.body &&
								typeof error.body === "object" &&
								"message" in error.body
							? String(error.body.message)
							: error.message}
				</Title>
			</Header>
			<Content>
				{"statusCode" in error && error.statusCode === 404
					? "The resource you are looking for does not exist or you do not have access to it."
					: 'description' in error ? <>{String(error.description)}</> : "An unexpected error occurred. Please try again later."}
			</Content>
			<Footer>
				<Button variant="secondary" onClick={resetError}>
					Retry
				</Button>
			</Footer>
		</>
	);
}
