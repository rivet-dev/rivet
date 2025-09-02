import type { ReactNode, RefObject } from "react";
import { createContext, useContext, useMemo } from "react";
import { assertNonNullable, type ImperativePanelHandle } from "@/components";

export interface RootLayoutContextValue {
	sidebarRef: RefObject<ImperativePanelHandle>;
	isSidebarCollapsed: boolean;
}

export const RootLayoutContext = createContext<RootLayoutContextValue | null>(
	null,
);

interface RootLayoutProviderProps extends RootLayoutContextValue {
	children: ReactNode;
}

export function RootLayoutContextProvider({
	children,
	sidebarRef,
	isSidebarCollapsed,
}: RootLayoutProviderProps) {
	return (
		<RootLayoutContext.Provider
			value={useMemo(
				() => ({
					sidebarRef,
					isSidebarCollapsed,
				}),
				[sidebarRef, isSidebarCollapsed],
			)}
		>
			{children}
		</RootLayoutContext.Provider>
	);
}

export function useRootLayout() {
	const context = useContext(RootLayoutContext);
	assertNonNullable(context);
	return context;
}
