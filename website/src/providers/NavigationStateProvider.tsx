"use client";

import { createContext, useContext, type ReactNode, useState, useCallback, useEffect } from "react";

interface NavigationStateContextType {
	isOpen: (itemId: string) => boolean;
	setIsOpen: (itemId: string, open: boolean) => void;
	toggleOpen: (itemId: string) => void;
	isHydrated: boolean;
}

const NavigationStateContext = createContext<NavigationStateContextType | undefined>(undefined);

export function useNavigationState() {
	const context = useContext(NavigationStateContext);
	// Return default no-op values during SSR when no provider is available
	if (!context) {
		return {
			isOpen: () => false,
			setIsOpen: () => {},
			toggleOpen: () => {},
			isHydrated: false,
		};
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
		// Default to true (open) for items that haven't been explicitly set
		return openStates[itemId] ?? true;
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
			// Default to true (open) if not set, so toggle will close it
			[itemId]: !(prev[itemId] ?? true),
		}));
	}, []);

	return (
		<NavigationStateContext.Provider
			value={{
				isOpen,
				setIsOpen,
				toggleOpen,
				isHydrated,
			}}
		>
			{children}
		</NavigationStateContext.Provider>
	);
}