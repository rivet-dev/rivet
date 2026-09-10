import * as DialogPrimitive from "@radix-ui/react-dialog";
import { faClose, Icon } from "@rivet-gg/icons";
import { QueryErrorResetBoundary } from "@tanstack/react-query";
import { type ComponentType, lazy, Suspense, useMemo } from "react";
import { ErrorBoundary } from "react-error-boundary";
import { cn, Frame, Skeleton, VisuallyHidden } from "@/components";
import dialogStyles from "@/components/ui/dialog.module.css";

// Match the settings drawer's `top: 56px` so all panels visually align with the
// top bar / chrome below it.
const TOP_BAR_OUTER_HEIGHT = "56px";

type FrameLoader = () => Promise<{
	default: ComponentType<{ onClose?: () => void }>;
}>;

// Each "Add provider" modal maps to its connect frame, rendered inside the
// right-side drawer instead of a centered dialog.
const PROVIDER_FRAMES: Record<string, FrameLoader> = {
	"connect-rivet": () => import("@/app/dialogs/connect-rivet-frame"),
	"connect-vercel": () => import("@/app/dialogs/connect-vercel-frame"),
	"connect-q-vercel": () =>
		import("@/app/dialogs/connect-quick-vercel-frame"),
	"connect-railway": () => import("@/app/dialogs/connect-railway-frame"),
	"connect-q-railway": () =>
		import("@/app/dialogs/connect-quick-railway-frame"),
	"connect-custom": () => import("@/app/dialogs/connect-manual-frame"),
	"connect-aws": () => import("@/app/dialogs/connect-aws-frame"),
	"connect-gcp": () => import("@/app/dialogs/connect-gcp-frame"),
	"connect-hetzner": () => import("@/app/dialogs/connect-hetzner-frame"),
};

export function isConnectProviderModal(modal: string | undefined): boolean {
	return typeof modal === "string" && modal in PROVIDER_FRAMES;
}

interface ConnectProviderSheetProps {
	modal: string | undefined;
	open: boolean;
	onOpenChange: (open: boolean) => void;
}

export function ConnectProviderSheet({
	modal,
	open,
	onOpenChange,
}: ConnectProviderSheetProps) {
	const Content = useMemo(() => {
		const loader = modal ? PROVIDER_FRAMES[modal] : undefined;
		return loader ? lazy(loader) : null;
	}, [modal]);

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
					<DialogPrimitive.Close
						className="absolute right-3 top-3 z-10 rounded-md p-1.5 text-muted-foreground hover:bg-foreground/[0.06] hover:text-foreground transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
						aria-label="Close provider setup"
					>
						<Icon icon={faClose} className="size-3.5" />
					</DialogPrimitive.Close>

					<div className="flex-1 min-h-0 flex flex-col gap-4 overflow-y-auto p-6">
						<Frame.IsInModalContext.Provider value={true}>
							<QueryErrorResetBoundary>
								{({ reset }) => (
									<ErrorBoundary
										onReset={reset}
										fallbackRender={({
											error,
											resetErrorBoundary,
										}) => (
											<div className="flex flex-col gap-3 rounded-md border border-destructive/30 bg-destructive/5 p-4 text-sm">
												<div className="font-medium text-destructive">
													Couldn't load provider
													setup.
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
												<>
													<VisuallyHidden>
														<DialogPrimitive.Title>
															Loading…
														</DialogPrimitive.Title>
													</VisuallyHidden>
													<div className="flex flex-col gap-4">
														<Skeleton className="h-6 w-1/3" />
														<Skeleton className="h-10 w-full" />
														<Skeleton className="h-24 w-full" />
														<Skeleton className="h-10 w-full" />
													</div>
												</>
											}
										>
											{Content ? (
												<Content
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
