/// <reference types="vite/client" />

declare const __APP_BUILD_ID__: string;
declare const __APP_TYPE__: "engine" | "inspector" | "cloud";
declare const Plain: {
	// This takes the same arguments as Plain.init. It will update the chat widget in-place with the new configuration.
	// Only top-level fields are updated, nested fields are not merged.
	update(params: any): void;

	// This takes the same arguments as `customerDetails` in Plain.init.
	// This will update just the customer details in the chat widget. This may be useful if you have asynchronous authentication state
	setCustomerDetails(params: any): void;

	// Opens and closes the widget if using the default, floating mode
	open(): void;
	close(): void;

	// These are event listeners that will be fired when the chat widget is opened or closed respectively
	// These return a function that can be called to remove the listener
	onOpen(callback: () => void): () => void;
	onClose(callback: () => void): () => void;

	// Returns whether or not the chat widget is initialized
	isInitialized(): boolean;

	// This returns an array with debug logs that have been collected by the chat widget
	// This is useful if you are contacting Plain support with an issue regarding the chat widget
	// This will redact sensitive information such as customer details
	exportDebugLogs(): any[];
};

declare module "*.module.css" {
	const classes: { [key: string]: string };
	export default classes;
}
