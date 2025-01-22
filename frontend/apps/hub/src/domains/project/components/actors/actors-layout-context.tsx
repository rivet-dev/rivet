import { assertNonNullable } from "@/lib/utils";
import type { ReactNode } from "react";
import { createContext, useContext, useMemo } from "react";

export interface ActorsLayoutContextValue {
	isFolded: boolean;
	setFolded: (value: boolean) => void;
}

export const ActorsLayoutContext =
	createContext<ActorsLayoutContextValue | null>(null);

interface ActorsLayoutProviderProps extends ActorsLayoutContextValue {
	children: ReactNode;
}

export function ActorsLayoutContextProvider({
	children,
	isFolded,
	setFolded,
}: ActorsLayoutProviderProps) {
	return (
		<ActorsLayoutContext.Provider
			value={useMemo(
				() => ({ isFolded, setFolded }),
				[isFolded, setFolded],
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
