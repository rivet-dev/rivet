/**
 * Core utility types following the Svelte 5 ecosystem convention
 * established by runed, melt-ui, and bits-ui.
 *
 * @module
 */

/** A function that returns a value of type `T`. */
export type Getter<T> = () => T;

/** A value of type `T`, or a getter function returning `T`. */
export type MaybeGetter<T> = T | Getter<T>;
