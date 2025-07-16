"use client";

import { createContext, useContext, type ReactNode, useState, useCallback, useEffect } from "react";

interface NavigationStateContextType {
	isOpen: (itemId: string) => boolean;
	setIsOpen: (itemId: string, open: boolean) => void;
	toggleOpen: (itemId: string) => void;
}

const NavigationStateContext = createContext<NavigationStateContextType | undefined>(undefined);

export function useNavigationState() {
	const context = useContext(NavigationStateContext);
	if (!context) {
		throw new Error("useNavigationState must be used within a NavigationStateProvider");
	}
	return context;
}

interface NavigationStateProviderProps {
	children: ReactNode;
}

const STORAGE_KEY = "rivet-navigation-state";

export function NavigationStateProvider({ children }: NavigationStateProviderProps) {
	const [openStates, setOpenStates] = useState<Record<string, boolean>>({});
	const [isHydrated, setIsHydrated] = useState(false);

	// Load from localStorage on mount
	useEffect(() => {
		try {
			const saved = localStorage.getItem(STORAGE_KEY);
			if (saved) {
				const parsed = JSON.parse(saved);
				setOpenStates(parsed);
			}
		} catch (error) {
			console.warn("Failed to load navigation state from localStorage:", error);
		}
		setIsHydrated(true);
	}, []);

	// Save to localStorage whenever state changes
	useEffect(() => {
		if (isHydrated) {
			try {
				localStorage.setItem(STORAGE_KEY, JSON.stringify(openStates));
			} catch (error) {
				console.warn("Failed to save navigation state to localStorage:", error);
			}
		}
	}, [openStates, isHydrated]);

	const isOpen = useCallback((itemId: string) => {
		return openStates[itemId] ?? false;
	}, [openStates]);

	const setIsOpen = useCallback((itemId: string, open: boolean) => {
		setOpenStates(prev => ({
			...prev,
			[itemId]: open,
		}));
	}, []);

	const toggleOpen = useCallback((itemId: string) => {
		setOpenStates(prev => ({
			...prev,
			[itemId]: !prev[itemId],
		}));
	}, []);

	return (
		<NavigationStateContext.Provider 
			value={{
				isOpen,
				setIsOpen,
				toggleOpen,
			}}
		>
			{children}
		</NavigationStateContext.Provider>
	);
}