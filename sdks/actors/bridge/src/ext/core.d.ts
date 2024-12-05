/**
 * This file contains types for internal Deno functionality.
 */

declare module "ext:core/mod.js" {
	export const core: {
		serialize(value: any, options?: { forStorage?: boolean }): Uint8Array;
		deserialize(value: Uint8Array, options?: { forStorage?: boolean }): any;
	};
	export const primordials: {
		ReflectOwnKeys: typeof Reflect.ownKeys;
		ObjectFreeze: typeof Object.freeze;
	};
}

declare module "ext:runtime/30_os.js" {
	export const env: {
		get(key: string): string?;
	};
}
