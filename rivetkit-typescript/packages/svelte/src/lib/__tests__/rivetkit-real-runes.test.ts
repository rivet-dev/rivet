import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { compileModule } from "svelte/compiler";
import { flushSync } from "svelte";
import * as ts from "typescript";
import { describe, expect, test } from "vitest";

const require = createRequire(import.meta.url);
const svelteInternalUrl = pathToFileURL(
  require.resolve("svelte/internal/client"),
).href;
const frameworkHarnessKey = "__rivetkit_svelte_real_runes_framework__";

function dataModule(source: string): string {
  return `data:text/javascript;base64,${Buffer.from(source).toString("base64")}`;
}

function compileRunesModule(source: string, filename: string): string {
  const { js } = compileModule(source, {
    filename,
    generate: "client",
    dev: false,
  });
  return js.code.replaceAll("svelte/internal/client", svelteInternalUrl);
}

function replaceModule(
  source: string,
  specifier: string,
  replacement: string,
): string {
  return source.replaceAll(`"${specifier}"`, JSON.stringify(replacement));
}

async function loadRealRunesHarness() {
  const frameworkStub = dataModule(`
    export function createRivetKit(_client, opts = {}) {
      const harness = globalThis[${JSON.stringify(frameworkHarnessKey)}];
      harness.configure(opts);
      return { getOrCreateActor: harness.getOrCreateActor };
    }
  `);
  const clientStub = dataModule(`
    export function createClient(input) { return input ?? {}; }
  `);
  const envStub = dataModule(`
    export const BROWSER = true;
    export const DEV = false;
  `);
  const extractStub = dataModule(`
    export function extract(value) {
      return typeof value === "function" ? value() : value;
    }
  `);
  const inspectorStub = dataModule(`
    export function createConnectionInspector() {
      throw new Error("connection inspector is disabled in this harness");
    }
  `);

  const sourcePath = resolve(process.cwd(), "src/lib/rivetkit.svelte.ts");
  const source = readFileSync(sourcePath, "utf8");
  let stripped = ts.transpileModule(source, {
    compilerOptions: {
      module: ts.ModuleKind.ESNext,
      target: ts.ScriptTarget.ES2022,
      verbatimModuleSyntax: true,
    },
    fileName: sourcePath,
  }).outputText;
  stripped = replaceModule(
    stripped,
    "./internal/framework-base.js",
    frameworkStub,
  );
  stripped = replaceModule(stripped, "rivetkit/client", clientStub);
  stripped = replaceModule(stripped, "esm-env", envStub);
  stripped = replaceModule(stripped, "./internal/extract.js", extractStub);
  stripped = replaceModule(
    stripped,
    "./connection-inspector.svelte.js",
    inspectorStub,
  );

  const adapterUrl = dataModule(compileRunesModule(stripped, sourcePath));
  const harnessSource = `
    import { createRivetKitWithClient } from ${JSON.stringify(adapterUrl)};

    export function createHarness() {
      let token = $state("token-1");
      let actor;
      const dispose = $effect.root(() => {
        const rivet = createRivetKitWithClient({}, {
          hashFunction: ({ name, key }) => JSON.stringify({ name, key })
        });
        actor = rivet.useActor(() => ({
          name: "chat",
          key: ["room-1"],
          params: { token },
          actionDefaults: {}
        }));
      });
      return {
        get actor() { return actor; },
        setToken(value) { token = value; },
        dispose
      };
    }
  `;
  const harnessUrl = dataModule(
    compileRunesModule(harnessSource, "rivetkit-real-runes-harness.svelte.js"),
  );
  return import(harnessUrl);
}

describe("useActor with the real Svelte rune runtime", () => {
  test("connected state does not feed the lifecycle effect back into itself", async () => {
    let getOrCreateCalls = 0;
    const connection = {
      increment: async (amount: number) => amount + 1,
      on: () => () => {},
    };
    let hashFunction = (opts: Record<string, unknown>) => JSON.stringify(opts);
    const state = {
      connection,
      handle: {},
      connStatus: "connected",
      error: null,
      hash: "stable",
    };
    const harness = {
      configure(opts: {
        hashFunction?: (value: Record<string, unknown>) => string;
      }) {
        hashFunction = opts.hashFunction ?? hashFunction;
      },
      getOrCreateActor(opts: Record<string, unknown>) {
        getOrCreateCalls += 1;
        return {
          key: hashFunction({ ...opts, enabled: opts.enabled ?? true }),
          mount: () => () => {},
          state: { state, subscribe: () => () => {} },
        };
      },
    };
    Object.assign(globalThis, { [frameworkHarnessKey]: harness });

    try {
      const { createHarness } = await loadRealRunesHarness();
      const instance = createHarness();
      await Promise.resolve();
      flushSync();

      expect(getOrCreateCalls).toBe(1);
      expect(instance.actor.hasEverConnected).toBe(true);

      instance.dispose();
      await Promise.resolve();
    } finally {
      delete (globalThis as Record<string, unknown>)[frameworkHarnessKey];
    }
  });

  test("same-hash option refresh retains an in-flight connection waiter", async () => {
    type Subscriber = (value: { currentVal: typeof state }) => void;
    const subscribers = new Set<Subscriber>();
    const connection = {
      increment: async (amount: number) => amount + 1,
      on: () => () => {},
    };
    let state = {
      connection,
      handle: {},
      connStatus: "connecting",
      error: null,
      hash: "stable",
    };
    let hashFunction = (opts: Record<string, unknown>) => JSON.stringify(opts);
    const harness = {
      configure(opts: {
        hashFunction?: (value: Record<string, unknown>) => string;
      }) {
        hashFunction = opts.hashFunction ?? hashFunction;
      },
      getOrCreateActor(opts: Record<string, unknown>) {
        return {
          key: hashFunction({ ...opts, enabled: opts.enabled ?? true }),
          mount: () => () => {},
          state: {
            get state() {
              return state;
            },
            subscribe(callback: Subscriber) {
              subscribers.add(callback);
              return () => subscribers.delete(callback);
            },
          },
        };
      },
    };
    Object.assign(globalThis, { [frameworkHarnessKey]: harness });

    try {
      const { createHarness } = await loadRealRunesHarness();
      const instance = createHarness();
      await Promise.resolve();
      flushSync();

      const pending = instance.actor.increment(1);
      expect(instance.actor.pendingActions).toBe(1);
      instance.setToken("token-2");
      flushSync();
      expect(instance.actor.pendingActions).toBe(1);

      state = { ...state, connStatus: "connected" };
      for (const subscriber of subscribers) {
        subscriber({ currentVal: state });
      }
      await expect(pending).resolves.toBe(2);
      expect(instance.actor.pendingActions).toBe(0);

      instance.dispose();
      await Promise.resolve();
    } finally {
      delete (globalThis as Record<string, unknown>)[frameworkHarnessKey];
    }
  });
});
