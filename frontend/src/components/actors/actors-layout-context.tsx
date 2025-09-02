import type { ReactNode, RefObject } from "react";
import { createContext, useContext, useMemo } from "react";
import { assertNonNullable } from "../lib/utils";
import type { ImperativePanelHandle } from "../ui/resizable";

export interface ActorsLayoutContextValue {
	detailsRef: RefObject<ImperativePanelHandle>;
	isDetailsColCollapsed: boolean;
}

export const ActorsLayoutContext =
	createContext<ActorsLayoutContextValue | null>(null);

interface ActorsLayoutProviderProps extends ActorsLayoutContextValue {
	children: ReactNode;
}

export function ActorsLayoutContextProvider({
	children,
	isDetailsColCollapsed,
	detailsRef,
}: ActorsLayoutProviderProps) {
	return (
		<ActorsLayoutContext.Provider
			value={useMemo(
				() => ({
					detailsRef,
					isDetailsColCollapsed,
				}),
				[detailsRef, isDetailsColCollapsed],
			)}
		>
			{children}
		</ActorsLayoutContext.Provider>
	);
}

export function useActorsLayout() {
	const context = useContext(ActorsLayoutContext);
	assertNonNullable(context);
	return context;
}
