import { memo, type ReactNode, Suspense, useRef, useState } from "react";
import { cn } from "../lib/utils";
import {
	type ImperativePanelHandle,
	ResizableHandle,
	ResizablePanel,
	ResizablePanelGroup,
} from "../ui/resizable";
import { ActorsLayoutContextProvider } from "./actors-layout-context";
import { ActorsListPanel } from "./actors-list-panel";
import { useRootLayout } from "./root-layout-context";

interface ActorsListPreviewProps {
	children: ReactNode;
	showDetails?: boolean;
}

export const ActorsListPreview = memo(
	({ children, showDetails }: ActorsListPreviewProps) => {
		const detailsColRef = useRef<ImperativePanelHandle>(null);

		const { isSidebarCollapsed } = useRootLayout();
		const [isDetailsColCollapsed, setIsDetailsColCollapsed] =
			useState(false);

		return (
			<ActorsLayoutContextProvider
				detailsRef={detailsColRef}
				isDetailsColCollapsed={isDetailsColCollapsed}
			>
				<ResizablePanelGroup
					direction="horizontal"
					autoSaveId="rivet-engine"
				>
					<ResizablePanel
						defaultSize={35}
						minSize={35}
						className="flex"
					>
						<div
							className={cn(
								"flex-1 min-h-0 flex overflow-hidden transition-colors",
								!isSidebarCollapsed &&
									"border-y border-x my-2 mr-2 rounded-lg bg-card",
							)}
						>
							<ActorsListPanel />
						</div>
					</ResizablePanel>

					{showDetails ? (
						<>
							<ResizableHandle
								className={cn(
									!isSidebarCollapsed &&
										"my-8 bg-transparent",
								)}
								onDoubleClick={() => {
									if (detailsColRef.current?.isCollapsed()) {
										detailsColRef.current?.expand();
									} else {
										detailsColRef.current?.collapse();
									}
								}}
							/>
							<ResizablePanel
								minSize={50}
								collapsible
								onCollapse={() =>
									setIsDetailsColCollapsed(true)
								}
								onExpand={() => setIsDetailsColCollapsed(false)}
								ref={detailsColRef}
								className="flex"
							>
								<div
									className={cn(
										"flex-1 overflow-hidden flex flex-col flex-grow transition-colors ",
										!isSidebarCollapsed &&
											"border-t border-b border-r border-l my-2 bg-card rounded-lg mr-2",
									)}
								>
									<Suspense>{children}</Suspense>
								</div>
							</ResizablePanel>
						</>
					) : null}
				</ResizablePanelGroup>
			</ActorsLayoutContextProvider>
		);
	},
);
