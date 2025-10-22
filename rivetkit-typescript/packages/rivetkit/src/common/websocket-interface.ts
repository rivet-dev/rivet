// Define minimal event interfaces to avoid conflicts between different WebSocket implementations
export interface RivetEvent {
	type: string;
	target?: any;
	currentTarget?: any;
}

export interface RivetMessageEvent extends RivetEvent {
	data: any;
}

export interface RivetCloseEvent extends RivetEvent {
	code: number;
	reason: string;
	wasClean: boolean;
}

/**
 * Common WebSocket interface that can be implemented by different WebSocket-like classes
 * This is compatible with the standard WebSocket API but allows for custom implementations
 */
export interface UniversalWebSocket {
	// WebSocket readyState values
	readonly CONNECTING: 0;
	readonly OPEN: 1;
	readonly CLOSING: 2;
	readonly CLOSED: 3;

	// Properties
	readonly readyState: 0 | 1 | 2 | 3;
	binaryType: "arraybuffer" | "blob";
	readonly bufferedAmount: number;
	readonly extensions: string;
	readonly protocol: string;
	readonly url: string;

	// Methods
	send(data: string | ArrayBufferLike | Blob | ArrayBufferView): void;
	close(code?: number, reason?: string): void;
	addEventListener(type: string, listener: (event: any) => void): void;
	removeEventListener(type: string, listener: (event: any) => void): void;
	dispatchEvent(event: RivetEvent): boolean;

	// Event handlers (optional)
	onopen?: ((event: RivetEvent) => void) | null;
	onclose?: ((event: RivetCloseEvent) => void) | null;
	onerror?: ((event: RivetEvent) => void) | null;
	onmessage?: ((event: RivetMessageEvent) => void) | null;
}
