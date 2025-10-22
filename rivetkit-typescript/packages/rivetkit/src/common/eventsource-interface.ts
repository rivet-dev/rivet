// Define minimal event interfaces to avoid conflicts between different EventSource implementations
export interface UniversalEvent {
	type: string;
	target?: any;
	currentTarget?: any;
}

export interface UniversalMessageEvent extends UniversalEvent {
	data: string;
	lastEventId: string;
	origin: string;
}

export interface UniversalErrorEvent extends UniversalEvent {
	message: string;
	filename?: string;
	lineno?: number;
	colno?: number;
	error?: any;
}

/**
 * Common EventSource interface that can be implemented by different EventSource-like classes
 * This is compatible with the standard EventSource API but allows for custom implementations
 */
export interface UniversalEventSource {
	// EventSource readyState values
	readonly CONNECTING: 0;
	readonly OPEN: 1;
	readonly CLOSED: 2;

	// Properties
	readonly readyState: 0 | 1 | 2;
	readonly url: string;
	readonly withCredentials: boolean;

	// Methods
	close(): void;
	addEventListener(type: string, listener: (event: any) => void): void;
	removeEventListener(type: string, listener: (event: any) => void): void;
	dispatchEvent(event: UniversalEvent): boolean;

	// Event handlers (optional)
	onopen?: ((event: UniversalEvent) => void) | null;
	onmessage?: ((event: UniversalMessageEvent) => void) | null;
	onerror?: ((event: UniversalErrorEvent) => void) | null;
}
