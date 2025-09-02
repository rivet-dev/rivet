import { useStore } from "@tanstack/react-store";
import { Store } from "@tanstack/store";

// Store state interface
interface ShiftKeyState {
	isShiftPressed: boolean;
}

// Initial state
const initialState: ShiftKeyState = {
	isShiftPressed: false,
};

// Create the store
export const shiftKeyStore = new Store(initialState);

// Store actions
export const shiftKeyActions = {
	setShiftPressed: (pressed: boolean) => {
		shiftKeyStore.setState((state) => ({
			...state,
			isShiftPressed: pressed,
		}));
	},
};

// Helper function to initialize event listeners
let isInitialized = false;

export const initializeShiftKeyTracking = () => {
	if (isInitialized) return;

	const handleKeyDown = (event: KeyboardEvent) => {
		if (event.key === "Shift") {
			shiftKeyActions.setShiftPressed(true);
		}
	};

	const handleKeyUp = (event: KeyboardEvent) => {
		if (event.key === "Shift") {
			shiftKeyActions.setShiftPressed(false);
		}
	};

	const handleWindowBlur = () => {
		// Reset shift state when window loses focus to avoid stuck keys
		shiftKeyActions.setShiftPressed(false);
	};

	const handleVisibilityChange = () => {
		// Reset shift state when tab becomes hidden
		if (document.hidden) {
			shiftKeyActions.setShiftPressed(false);
		}
	};

	// Add event listeners
	document.addEventListener("keydown", handleKeyDown);
	document.addEventListener("keyup", handleKeyUp);
	window.addEventListener("blur", handleWindowBlur);
	document.addEventListener("visibilitychange", handleVisibilityChange);

	isInitialized = true;

	// Return cleanup function
	return () => {
		document.removeEventListener("keydown", handleKeyDown);
		document.removeEventListener("keyup", handleKeyUp);
		window.removeEventListener("blur", handleWindowBlur);
		document.removeEventListener(
			"visibilitychange",
			handleVisibilityChange,
		);
		isInitialized = false;
	};
};

// React hook for easy consumption
export const useShiftKeyStore = () => {
	return useStore(shiftKeyStore);
};

// Selector for just the shift state
export const useIsShiftPressed = () => {
	return useStore(shiftKeyStore, (state) => state.isShiftPressed);
};
