# Workflow Friction Log

Issues encountered while building workflow examples with RivetKit. These are friction points that make the developer experience harder than it should be.

## Type System Issues

### 1. Loop context typed as `WorkflowContextInterface` instead of `ActorWorkflowContext`

**Problem:** When using `ctx.loop()`, the callback receives a `WorkflowContextInterface` parameter, but the actual runtime type is `ActorWorkflowContext` which has additional properties like `state`, `broadcast`, `vars`, `log`, etc.

**Symptom:**
```typescript
await ctx.loop({
  name: "my-loop",
  run: async (loopCtx) => {
    // TypeScript error: Property 'state' does not exist on type 'WorkflowContextInterface'
    const item = loopCtx.state.items.find(i => i.id === id);

    // TypeScript error: Property 'broadcast' does not exist on type 'WorkflowContextInterface'
    loopCtx.broadcast("itemUpdated", item);
  }
});
```

**Workaround:** Create helper functions to cast the context:
```typescript
const getState = <S>(ctx: unknown): S => (ctx as { state: S }).state;
const getBroadcast = (ctx: unknown) =>
  (ctx as { broadcast: (name: string, ...args: unknown[]) => void }).broadcast;

// Usage inside loop
const state = getState<MyStateType>(loopCtx);
const broadcast = getBroadcast(loopCtx);
```

**Impact:** Every workflow example needs boilerplate type helpers. State types must be defined separately and cannot be inferred from the actor definition.

### 2. No way to access actor state type from actor definition

**Problem:** There's no `._` property or similar mechanism to extract the state type from an actor definition.

**What doesn't work:**
```typescript
// Doesn't work - '._' does not exist
const state = getState<typeof myActor._.state>(loopCtx);
```

**Workaround:** Define state types separately:
```typescript
type MyActorState = { items: Item[] };

export const myActor = actor({
  state: { items: [] as Item[] },
  // ...
});

// In workflow
const state = getState<MyActorState>(loopCtx);
```

**Impact:** State type definitions are duplicated - once in the actor definition and once as a standalone type.

## Missing Methods

### 3. `ActorQueue` was missing `send()` method

**Problem:** The `ActorQueue` class only had `next()` for receiving messages, but no method to send messages to queues.

**Error:**
```typescript
// Property 'send' does not exist on type 'ActorQueue<...>'
await c.queue.send(QUEUE_NAME, { orderId });
```

**Fix:** Added `send()` method to `ActorQueue` class:
```typescript
async send(name: string, body: unknown): Promise<QueueMessage> {
  return await this.#queueManager.enqueue(name, body);
}
```

### 4. `ActorWorkflowContext` was missing `broadcast()` method

**Problem:** The workflow context wrapper didn't expose the `broadcast()` method from the underlying run context.

**Error:**
```typescript
// Property 'broadcast' does not exist on type 'ActorWorkflowContext<...>'
ctx.broadcast("orderUpdated", order);
```

**Fix:** Added `broadcast()` method that delegates to `this.#runCtx.broadcast()`.

## Package Resolution Issues

### 5. `@rivetkit/workflow-engine` not found on npm

**Problem:** Examples using `Loop` from `@rivetkit/workflow-engine` failed with 404 error because the package isn't published to npm.

**Error:**
```
ERR_PNPM_FETCH_404 GET https://registry.npmjs.org/@rivetkit/workflow-engine - Not Found
```

**Workaround:** Re-export `Loop` from `rivetkit/workflow`:
```typescript
// In rivetkit/workflow/mod.ts
export { Loop } from "@rivetkit/workflow-engine";

// In examples
import { Loop, workflow } from "rivetkit/workflow";
```

**Impact:** Internal packages need to be re-exported through public packages.

### 6. Need to understand pnpm workspace resolutions for examples

**Problem:** Examples need to use `*` as version for workspace packages, and the root `package.json` needs `resolutions` entries with `workspace:*`.

**What doesn't work:**
```json
{
  "dependencies": {
    "rivetkit": "workspace:*"  // Doesn't work in examples
  }
}
```

**What works:**
```json
// In example package.json
{
  "dependencies": {
    "rivetkit": "*"
  }
}

// In root package.json
{
  "resolutions": {
    "rivetkit": "workspace:*"
  }
}
```

## Build Issues

### 7. `workflow/mod.ts` return type incompatible with `RunConfig`

**Problem:** The `run` function type in `workflow()` wasn't compatible with `RunConfig`'s expected type.

**Error:**
```
TS2322: Type '(...) => Promise<never>' is not assignable to type '(...) => unknown'
```

**Fix:** Cast the return value:
```typescript
return {
  icon: "diagram-project",
  run: run as (...args: unknown[]) => unknown,
};
```

### 8. Invalid example tag "workflows"

**Problem:** The `template.tags` field in package.json had an invalid tag value.

**Error:**
```
Invalid tag "workflows"
```

**Fix:** Changed to valid tag "experimental".

## Frontend Integration Issues

### 9. `actor.connection` is possibly null

**Problem:** The useActor hook returns a connection that can be null before the actor is connected.

**Symptom:** Every connection method call needs null checking:
```typescript
// Error: actor.connection is possibly null
await actor.connection.createOrder(orderId);

// Fix
await actor.connection?.createOrder(orderId);
```

### 10. Union types in useActor break method access

**Problem:** When passing actor to child components, the type becomes a union of all possible actor connections, making it impossible to call actor-specific methods.

**Symptom:**
```typescript
// Type: ReturnType<typeof useActor>
// Property 'approve' does not exist on union type
actor.connection.approve(requestId, "Admin");
```

**Workaround:** Pass callback functions instead of the actor:
```typescript
// Instead of
<RequestCountdown actor={actor} />

// Use
<RequestCountdown
  onApprove={(id, approver) => actor.connection?.approve(id, approver)}
  onReject={(id, approver) => actor.connection?.reject(id, approver)}
/>
```

## Matchmaking Example Type Safety

### 11. `c.client<any>()` and `client: <T>() => any` in multiplayer examples remove actor-to-actor type safety

**Problem:** The `examples/multiplayer-game-patterns` actors use `any` for internal actor clients. This bypasses compile-time checks for action names and payload shapes in the most security-sensitive paths (lobby validation, join authorization, and lifecycle updates).

**Problematic locations:**
- `examples/multiplayer-game-patterns/src/actors/turn-based/match.ts:95`
- `examples/multiplayer-game-patterns/src/actors/turn-based/match.ts:112`
- `examples/multiplayer-game-patterns/src/actors/turn-based/match.ts:192`
- `examples/multiplayer-game-patterns/src/actors/ranked/match.ts:86`
- `examples/multiplayer-game-patterns/src/actors/ranked/match.ts:150`
- `examples/multiplayer-game-patterns/src/actors/competitive/match.ts:101`
- `examples/multiplayer-game-patterns/src/actors/competitive/match.ts:118`
- `examples/multiplayer-game-patterns/src/actors/competitive/match.ts:196`
- `examples/multiplayer-game-patterns/src/actors/battle-royale/match.ts:73`
- `examples/multiplayer-game-patterns/src/actors/battle-royale/match.ts:75`
- `examples/multiplayer-game-patterns/src/actors/battle-royale/match.ts:89`
- `examples/multiplayer-game-patterns/src/actors/battle-royale/match.ts:122`
- `examples/multiplayer-game-patterns/src/actors/io-style/match.ts:41`
- `examples/multiplayer-game-patterns/src/actors/io-style/match.ts:45`
- `examples/multiplayer-game-patterns/src/actors/io-style/match.ts:73`
- `examples/multiplayer-game-patterns/src/actors/io-style/match.ts:91`
- `examples/multiplayer-game-patterns/src/actors/io-style/match.ts:140`
- `examples/multiplayer-game-patterns/src/actors/party/match.ts:54`
- `examples/multiplayer-game-patterns/src/actors/party/match.ts:56`
- `examples/multiplayer-game-patterns/src/actors/party/match.ts:88`
- `examples/multiplayer-game-patterns/src/actors/party/match.ts:121`
- `examples/multiplayer-game-patterns/src/actors/party/match.ts:178`
- `examples/multiplayer-game-patterns/src/actors/open-world/world-index.ts:99`
- `examples/multiplayer-game-patterns/src/actors/open-world/world-index.ts:102`
- `examples/multiplayer-game-patterns/src/actors/open-world/chunk.ts:82`
- `examples/multiplayer-game-patterns/src/actors/open-world/chunk.ts:85`

**Impact:**
- API drift between matchmaker and match actors is not caught by TypeScript.
- Mistyped control/join/auth payload fields can compile and fail only at runtime.
- Security controls become easier to accidentally bypass through incorrect method names or payload shapes.
- Refactors are harder because IDE rename/type tooling cannot validate these call sites.

**Workaround used:** Keep `any` casts local and manually guard critical responses (`validateLobby` and `authorizeJoin`) before mutating state.

**Desired fix:** Expose a typed registry client for actor runtime contexts so examples can use `c.client<YourRegistryType>()` without `any`.

## Summary

The biggest pain points are:
1. **Type system gaps** - loop context doesn't have the right type, requiring manual type helpers
2. **Missing methods** - `send()` and `broadcast()` weren't exposed on expected interfaces
3. **Package structure** - internal packages not accessible, need re-exports through public API
4. **Actor client typing gaps** - actor-to-actor calls in multiplayer examples require `any`, removing compile-time safety in auth-critical flows

These issues could be addressed by:
- Making `ActorWorkflowContext` the declared type for loop callbacks
- Exposing a type helper like `actor.State` or `typeof actor['state']`
- Ensuring all expected methods are on the public interfaces
- Re-exporting internal types/values through the main package
