/// <reference types="vite/client" />

declare const __APP_BUILD_ID__: string;
declare const __APP_TYPE__: "engine" | "inspector" | "cloud";

declare module "*.module.css" {
	const classes: { [key: string]: string };
	export default classes;
}
