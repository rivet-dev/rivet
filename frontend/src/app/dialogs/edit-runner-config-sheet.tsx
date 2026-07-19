import * as DialogPrimitive from "@radix-ui/react-dialog";
import { faClose, Icon } from "@rivet-gg/icons";
import { QueryErrorResetBoundary } from "@tanstack/react-query";
import { lazy, Suspense } from "react";
import { ErrorBoundary } from "react-error-boundary";
import { cn, Frame, Skeleton } from "@/components";
import dialogStyles from "@/components/ui/dialog.module.css";

const EditRunnerConfigFrameContent = lazy(
	() => import("@/app/dialogs/edit-runner-config"),
);

// Match the settings drawer's `top: 56px` so both panels visually align with
// the top bar / chrome below it.
const TOP_BAR_OUTER_HEIGHT = "56px";

interface EditRunnerConfigSheetProps {
	open: boolean;
	onOpenChange: (open: boolean) => void;
	name?: string;
	dc?: string;
}

export function EditRunnerConfigSheet({
	open,
	onOpenChange,
	name,
	dc,
}: EditRunnerConfigSheetProps) {
	return (
		<DialogPrimitive.Root open={open} onOpenChange={onOpenChange} modal>
			<DialogPrimitive.Portal>
				<DialogPrimitive.Overlay
					className={cn(
						"fixed inset-x-0 bottom-0 z-[55] backdrop-blur-[1px]",
						dialogStyles.overlay,
					)}
					style={{ top: TOP_BAR_OUTER_HEIGHT }}
				/>
				<DialogPrimitive.Content
					className={cn(
						"fixed right-2 z-[60] w-[520px] max-w-[calc(100vw-1rem)] flex flex-col overflow-hidden",
						"bg-card border border-border rounded-lg shadow-xl",
						"focus:outline-none",
						"data-[state=open]:animate-in data-[state=closed]:animate-out",
						"data-[state=open]:slide-in-from-right data-[state=closed]:slide-out-to-right",
						"data-[state=open]:duration-200 data-[state=closed]:duration-150",
					)}
					style={{ top: TOP_BAR_OUTER_HEIGHT, bottom: "8px" }}
				>
					<header className="shrink-0 flex items-center justify-between gap-3 px-5 h-12 border-b border-border">
						<DialogPrimitive.Title className="flex min-w-0 items-center gap-2 text-sm font-semibold text-foreground">
							<span className="truncate">Edit provider</span>
							{name ? (
								<span className="shrink-0 rounded border border-border bg-foreground/[0.04] px-1.5 py-0.5 font-mono-console text-xs font-normal text-muted-foreground">
									{name}
								</span>
							) : null}
						</DialogPrimitive.Title>
						<DialogPrimitive.Close
							className="shrink-0 rounded-md p-1.5 text-muted-foreground hover:bg-foreground/[0.06] hover:text-foreground transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
							aria-label="Close provider settings"
						>
							<Icon icon={faClose} className="size-3.5" />
						</DialogPrimitive.Close>
					</header>

					<div className="flex-1 min-h-0 flex flex-col">
						<Frame.IsInModalContext.Provider value={true}>
							<QueryErrorResetBoundary>
								{({ reset }) => (
									<ErrorBoundary
										onReset={reset}
										fallbackRender={({
											error,
											resetErrorBoundary,
										}) => (
											<div className="m-5 flex flex-col gap-3 rounded-md border border-destructive/30 bg-destructive/5 p-4 text-sm">
												<div className="font-medium text-destructive">
													Couldn't load provider
													settings.
												</div>
												<div className="text-muted-foreground text-xs">
													{error?.message ??
														"Unknown error"}
												</div>
												<button
													type="button"
													className="self-start text-xs underline-offset-2 hover:underline"
													onClick={resetErrorBoundary}
												>
													Retry
												</button>
											</div>
										)}
									>
										<Suspense
											fallback={
												<div className="flex flex-col gap-4 p-5">
													<Skeleton className="h-9 w-full" />
													<Skeleton className="h-10 w-full" />
													<Skeleton className="h-24 w-full" />
													<Skeleton className="h-10 w-full" />
												</div>
											}
										>
											{name ? (
												<EditRunnerConfigFrameContent
													name={name}
													dc={dc}
													onClose={() =>
														onOpenChange(false)
													}
												/>
											) : null}
										</Suspense>
									</ErrorBoundary>
								)}
							</QueryErrorResetBoundary>
						</Frame.IsInModalContext.Provider>
					</div>
				</DialogPrimitive.Content>
			</DialogPrimitive.Portal>
		</DialogPrimitive.Root>
	);
}
