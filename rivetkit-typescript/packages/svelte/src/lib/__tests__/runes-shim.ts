type EffectEntry = {
  run: () => unknown;
  cleanup?: () => void;
};

const effects = new Set<EffectEntry>();

function runEffect(entry: EffectEntry): void {
  entry.cleanup?.();
  const cleanup = entry.run();
  entry.cleanup =
    typeof cleanup === "function" ? (cleanup as () => void) : undefined;
}

const effect = ((fn?: () => unknown) => {
  if (!fn) return;
  const entry: EffectEntry = { run: fn };
  effects.add(entry);
  runEffect(entry);
}) as unknown as {
  (fn?: () => unknown): unknown;
  root: (fn: () => void | (() => void)) => () => void;
};

effect.root = (fn) => {
  const cleanup = fn();
  return typeof cleanup === "function" ? cleanup : () => {};
};

const state = ((value?: unknown) => value) as unknown as typeof $state;
(state as unknown as { raw: <T>(value: T) => T }).raw = <T>(value: T) => value;

const derived = {
  by: <T>(fn: () => T): T => fn(),
};

(globalThis as Record<string, unknown>).$state = state;
(globalThis as Record<string, unknown>).$effect = effect;
(globalThis as Record<string, unknown>).$derived = derived;

/** Re-run registered effects after their cleanup, simulating a dependency change. */
export function rerunEffects(): void {
  for (const entry of effects) runEffect(entry);
}

/** Dispose and forget effects registered by the previous test. */
export function resetEffects(): void {
  for (const entry of effects) entry.cleanup?.();
  effects.clear();
}
