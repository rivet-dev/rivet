# Dynamic Actor TypeScript Source Compilation

## Overview

Add TypeScript source support to dynamic actors via `@secure-exec/typescript`, allowing loaders to return `.ts` source directly instead of requiring pre-transpiled JavaScript.

## Current State

- `DynamicActorLoadResult.sourceFormat` accepts `"commonjs-js" | "esm-js"` only
- Loaders must pre-transpile TypeScript before returning source
- secure-exec is loaded dynamically at runtime (not a direct dependency) from `secure-exec` or the legacy `sandboxed-node` package specifier
- The codebase currently resolves secure-exec from a pre-release commit hash (`pkg.pr.new/rivet-dev/secure-exec@7659aba`) in the example, and from local dist paths or npm in the runtime

## Dependency Update

Update secure-exec from the pre-release commit hash to the published `0.1.0` release:

- `secure-exec@0.1.0` — core runtime (published 2026-03-18)
- `@secure-exec/typescript@0.1.0` — TypeScript compiler tools (published 2026-03-18, depends on `secure-exec@0.1.0` and `typescript@^5.9.3`)

The `@secure-exec/typescript` package provides `createTypeScriptTools()` which runs the TypeScript compiler inside a secure-exec isolate, returning compiled JS and diagnostics. This means type-checking and transpilation happen in a sandboxed environment with memory/CPU limits, matching the existing security model.

Update locations:
- `examples/ai-generated-actor/package.json` — replace commit hash URL with `secure-exec@0.1.0`
- Any local dist path fallbacks in `isolate-runtime.ts` that reference old directory structures

## New Source Formats

Extend `DynamicSourceFormat` in `runtime-bridge.ts`:

```ts
export type DynamicSourceFormat =
  | "commonjs-js"
  | "esm-js"
  | "esm-ts"       // ESM TypeScript, compiled to esm-js before execution
  | "commonjs-ts";  // CJS TypeScript, compiled to commonjs-js before execution
```

## API: `compileActorSource`

Exported from `rivetkit/dynamic`. This is a helper that the loader calls explicitly — compilation does not happen implicitly in the runtime.

### Signature

```ts
interface CompileActorSourceOptions {
  /** TypeScript source text. */
  source: string;

  /** Filename hint for diagnostics (default: "actor.ts"). */
  filename?: string;

  /** Output module format (default: "esm"). */
  format?: "esm" | "commonjs";

  /** Run the full type checker (default: false). Strip-only when false. */
  typecheck?: boolean;

  /** Additional tsconfig compilerOptions overrides. */
  compilerOptions?: Record<string, unknown>;

  /** Memory limit for the compiler isolate in MB (default: 512). */
  memoryLimit?: number;

  /** CPU time limit for the compiler isolate in ms. */
  cpuTimeLimitMs?: number;
}

interface CompileActorSourceResult {
  /** Compiled JavaScript output. Undefined if compilation failed. */
  js?: string;

  /** Source map text, if generated. */
  sourceMap?: string;

  /** Whether compilation succeeded without errors. */
  success: boolean;

  /** TypeScript diagnostics (errors, warnings, suggestions). */
  diagnostics: TypeScriptDiagnostic[];
}

interface TypeScriptDiagnostic {
  code: number;
  category: "error" | "warning" | "suggestion" | "message";
  message: string;
  line?: number;
  column?: number;
}

function compileActorSource(
  options: CompileActorSourceOptions,
): Promise<CompileActorSourceResult>;
```

### Usage in a Loader

```ts
import { dynamicActor, compileActorSource } from "rivetkit/dynamic";

const myActor = dynamicActor({
  load: async (ctx) => {
    const tsSource = await fetchActorSource(ctx.name);

    const compiled = await compileActorSource({
      source: tsSource,
      typecheck: true,
    });

    if (!compiled.success) {
      const errors = compiled.diagnostics
        .filter(d => d.category === "error")
        .map(d => `${d.line}:${d.column} ${d.message}`)
        .join("\n");
      throw new Error(`Actor TypeScript compilation failed:\n${errors}`);
    }

    return {
      source: compiled.js!,
      sourceFormat: "esm-js",
    };
  },
});
```

### Usage Without Type Checking (Fast Path)

```ts
const compiled = await compileActorSource({
  source: tsSource,
  typecheck: false, // strip types only, much faster
});
```

## Implementation Plan

### 1. Update secure-exec dependency

- Replace pre-release URLs with `secure-exec@0.1.0` in examples
- Add `@secure-exec/typescript` as an optional peer dependency of rivetkit (dynamically loaded like secure-exec itself)

### 2. Add `compileActorSource` to `rivetkit/dynamic`

New file: `src/dynamic/compile.ts`

Implementation:
1. Dynamically load `@secure-exec/typescript` (same pattern as secure-exec itself — build specifier from parts to avoid bundler eager inclusion)
2. Dynamically load `secure-exec` to get `SystemDriver` and `NodeRuntimeDriverFactory`
3. Call `createTypeScriptTools()` with the secure-exec drivers
4. Call `compileSource()` with the user's source text and compiler options
5. Map the `SourceCompileResult` to `CompileActorSourceResult`

The key mapping from `@secure-exec/typescript` API to ours:

| `@secure-exec/typescript`        | `compileActorSource`               |
|-----------------------------------|------------------------------------|
| `createTypeScriptTools()`         | Called internally, cached per call |
| `tools.compileSource()`           | Core operation                     |
| `tools.typecheckSource()`         | Used when `typecheck: true`        |
| `SourceCompileResult.outputText`  | `CompileActorSourceResult.js`      |
| `SourceCompileResult.diagnostics` | Passed through directly            |

When `typecheck: false`, use compiler options `{ noCheck: true }` (TS 5.9+ `--noCheck` flag) to strip types without running the checker. This is substantially faster.

### 3. Add source format aliases (optional convenience)

Extend `DynamicSourceFormat` with `"esm-ts"` and `"commonjs-ts"`. When the isolate runtime sees a TS format, it calls `compileActorSource` automatically before writing source to the sandbox filesystem. This is a convenience — loaders can always compile explicitly and return `"esm-js"`.

### 4. Export from `rivetkit/dynamic`

Add to `src/dynamic/mod.ts`:
```ts
export { compileActorSource } from "./compile";
export type {
  CompileActorSourceOptions,
  CompileActorSourceResult,
  TypeScriptDiagnostic,
} from "./compile";
```

### 5. Tests

- Unit test: `compileActorSource` with valid TS returns JS and `success: true`
- Unit test: `compileActorSource` with type errors returns diagnostics and `success: false`
- Unit test: `compileActorSource` with `typecheck: false` strips types without error on invalid types
- Driver test: dynamic actor with `sourceFormat: "esm-ts"` loads and responds to actions
- Driver test: dynamic actor reload with TS source

## Design Decisions

**Why a helper function, not automatic compilation in reload/load?**
- Type checking is expensive (spins up a compiler isolate). Loaders should opt in explicitly.
- Loaders may want to cache compiled output, skip type checking in production, or use different compiler options per actor.
- Keeps the runtime path simple — it always receives JS.

**Why not `transpile` or `prepare`?**
- `compile` is the standard term in the TypeScript ecosystem for TS→JS transformation.
- `transpile` is technically more precise but less commonly used by TS developers.
- `prepare` is too vague.

**Why run the compiler inside secure-exec?**
- `@secure-exec/typescript` already handles this — the compiler runs in an isolate with memory/CPU limits.
- User-provided source code never touches the host TypeScript installation.
- Consistent with the existing security model where all dynamic actor code runs sandboxed.

## Files Changed

| File | Change |
|------|--------|
| `src/dynamic/compile.ts` | New — `compileActorSource` implementation |
| `src/dynamic/mod.ts` | Export `compileActorSource` and types |
| `src/dynamic/runtime-bridge.ts` | Add `"esm-ts"` and `"commonjs-ts"` to `DynamicSourceFormat` |
| `src/dynamic/isolate-runtime.ts` | Handle TS formats by compiling before sandbox write |
| `examples/ai-generated-actor/package.json` | Update secure-exec to `0.1.0` |
