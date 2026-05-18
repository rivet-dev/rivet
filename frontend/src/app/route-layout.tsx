import { Outlet } from "@tanstack/react-router";
import { useRef } from "react";
import type { ImperativePanelHandle } from "react-resizable-panels";
import { H2, Skeleton } from "@/components";
import { RootLayoutContextProvider } from "@/components/actors/root-layout-context";
import { TopBar } from "./top-bar";

export function RouteLayout({
	children = <Outlet />,
}: {
	children?: React.ReactNode;
}) {
	const sidebarRef = useRef<ImperativePanelHandle>(null);

	return (
		<div className="flex h-screen flex-col bg-background pl-2">
			<TopBar />
			<main className="flex flex-1 min-h-0 flex-col bg-background min-w-0">
				{/*
				 * `isSidebarCollapsed: false` so the inner Content / ActorsListPreview
				 * components still render their floating card border. There is no
				 * sidebar anymore (top bar replaced it), but the styling condition
				 * was inherited from the old layout.
				 */}
				<RootLayoutContextProvider
					sidebarRef={sidebarRef}
					isSidebarCollapsed={false}
				>
					{children}
				</RootLayoutContextProvider>
			</main>
		</div>
	);
}

export function PendingRouteLayout() {
	return (
		<RouteLayout>
			<div className="bg-card h-full border border-border mt-2 mb-2 ml-2 mr-2 rounded-lg">
				<div className="mt-2 flex justify-between items-center px-6 py-4">
					<H2 className="mb-2">
						<Skeleton className="w-48 h-8" />
					</H2>
				</div>
				<p className="max-w-5xl mb-6 px-6 text-muted-foreground">
					<Skeleton className="w-full h-4" />
				</p>
				<hr className="mb-4" />
				<div className="p-4 px-6 max-w-5xl ">
					<Skeleton className="h-8 w-48 mb-4" />
					<div className="flex flex-wrap gap-2 my-4">
						<Skeleton className="min-w-48 h-auto min-h-28 rounded-md" />
						<Skeleton className="min-w-48 h-auto min-h-28 rounded-md" />
						<Skeleton className="min-w-48 h-auto min-h-28 rounded-md" />
					</div>
				</div>
			</div>
		</RouteLayout>
	);
}
