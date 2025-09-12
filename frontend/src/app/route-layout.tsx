import { Outlet } from "@tanstack/react-router";
import { useRef, useState } from "react";
import type { ImperativePanelHandle } from "react-resizable-panels";
import { RootLayoutContextProvider } from "@/components/actors/root-layout-context";
import * as Layout from "./layout";

export function RouteLayout({
	children = <Outlet />,
}: {
	children?: React.ReactNode;
}) {
	const sidebarRef = useRef<ImperativePanelHandle>(null);
	const [isSidebarCollapsed, setIsSidebarCollapsed] = useState(false);

	return (
		<Layout.Root>
			<Layout.VisibleInFull>
				<Layout.Sidebar
					ref={sidebarRef}
					onCollapse={() => {
						setIsSidebarCollapsed(true);
					}}
					onExpand={() => setIsSidebarCollapsed(false)}
				/>
				<Layout.Main>
					<RootLayoutContextProvider
						sidebarRef={sidebarRef}
						isSidebarCollapsed={isSidebarCollapsed}
					>
						{children}
					</RootLayoutContextProvider>
				</Layout.Main>
			</Layout.VisibleInFull>
			<Layout.Footer />
		</Layout.Root>
	);
}
