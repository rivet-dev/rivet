/// <reference types="vite/client" />

declare const __APP_BUILD_ID__: string;

declare module "*.module.css" {
	const classes: { [key: string]: string };
	export default classes;
}
