/**
 * This file contains types for internal Deno functionality.
 */

declare module "ext:core/mod.js" {
  export const core: {
    serialize(value: any, options?: { forStorage?: boolean }): Uint8Array;
    deserialize(value: Uint8Array, options?: { forStorage?: boolean }): any;
    propReadOnly(value: any): any;
  };
  export const primordials: {
    ObjectDefineProperty: typeof Object.defineProperty;
  };
}
