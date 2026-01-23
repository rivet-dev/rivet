/**
 * Node.js entrypoint for rivetkit.
 *
 * This file sets up Node.js dependencies (polyfills + injected deps)
 * before re-exporting the core rivetkit API.
 *
 * Used by package.json conditional exports for the "node" condition.
 */

// Side effect: injects polyfills and Node dependencies
import "./node-setup";

// Re-export everything from the core module
export * from "./mod";
