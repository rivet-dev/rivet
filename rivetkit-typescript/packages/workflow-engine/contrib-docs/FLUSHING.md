# Workflow Engine: Flushing & Persistence

This document explains how the workflow engine persists state and the critical role of `flush()`.

## Overview

The workflow engine uses an in-memory storage layer (`Storage`) that periodically writes to persistent KV storage via `flush()`. Understanding when and why to flush is critical for maintaining workflow durability.

## The Storage Model

```
┌─────────────────────────────────────────────────────────┐
│                    In-Memory Storage                     │
├─────────────────────────────────────────────────────────┤
│  nameRegistry[]     - Deduplicated strings for names    │
│  history.entries    - Map<key, Entry> workflow steps    │
│  entryMetadata      - Map<id, Metadata> retry info      │
│  messages[]         - Pending workflow messages         │
│  state              - pending/running/sleeping/etc      │
│  output/error       - Final workflow result             │
└─────────────────────────────────────────────────────────┘
                           │
                           │ flush()
                           ▼
┌─────────────────────────────────────────────────────────┐
│                   Persistent KV Storage                  │
├─────────────────────────────────────────────────────────┤
│  workflow:name:{idx}       - Name registry entries      │
│  workflow:history:{loc}    - Serialized Entry objects   │
│  workflow:meta:{id}        - Entry metadata             │
│  workflow:message:{id}     - Pending messages           │
│  workflow:state            - Workflow state enum        │
│  workflow:output           - Final output               │
│  workflow:error            - Error if failed            │
└─────────────────────────────────────────────────────────┘
```

## The Dirty Flag Pattern

Entries use a `dirty` flag to track what needs to be written:

```typescript
// 1. Create or modify an entry
entry.dirty = true;

// 2. Flush writes all dirty entries to KV
await flush(storage, driver);  // Sets dirty = false

// 3. Entry is now persisted and safe from crashes
```

## When to Flush

### The Golden Rule

**Flush immediately after any state change that must survive a crash.**

If the workflow could crash (or yield via `SleepError`) after a state change, that state must be flushed first.

### Current Flush Points

| Operation | When Flushed | Why |
|-----------|--------------|-----|
| Non-ephemeral step | After completion | Step results must persist |
| Loop iteration | After each iteration | Progress must be resumable |
| Sleep (past deadline) | After marking complete | Don't re-sleep on restart |
| Sleep (short, in-memory) | After completing | Same as above |
| Listen (messages consumed) | After consumption | Don't re-consume |
| Listen with timeout | After deadline created | Deadline must persist |
| Join entry creation | Before branches run | Entry structure must exist |
| Race entry creation | Before branches run | Entry structure must exist |
| Join/Race completion | After all branches | Final state must persist |
| Message consumption | After deletion | Prevent re-delivery |

### Ephemeral Steps

Ephemeral steps (`{ ephemeral: true }`) skip immediate flush for performance. They batch with the next non-ephemeral operation. Use only for:
- Idempotent operations
- Operations where replay is acceptable
- Performance-critical paths

## Common Bugs to Avoid

### Bug Pattern 1: Missing Flush Before Yield

```typescript
// BAD: Entry created but not flushed before potential SleepError
setEntry(storage, location, entry);
entry.dirty = true;
await somethingThatMightThrowSleepError();  // If this yields, entry is lost!

// GOOD: Flush before potential yield
setEntry(storage, location, entry);
entry.dirty = true;
await flush(storage, driver);  // Persist first
await somethingThatMightThrowSleepError();  // Safe to yield now
```

### Bug Pattern 2: Missing Flush Before Branching

```typescript
// BAD: Parent entry not flushed before children run
entry = createEntry(location, { type: "join", data: {...} });
setEntry(storage, location, entry);
entry.dirty = true;
// Start branches immediately - if one fails fast, parent entry is lost!
await Promise.all(branches.map(b => b.run()));

// GOOD: Flush parent before children
entry.dirty = true;
await flush(storage, driver);  // Parent entry persisted
await Promise.all(branches.map(b => b.run()));  // Safe to run
```

### Bug Pattern 3: Missing Flush for In-Memory Completion

```typescript
// BAD: Short sleep completes in memory but state not persisted
if (remaining < workerPollInterval) {
    await sleep(remaining);
    entry.kind.data.state = "completed";
    entry.dirty = true;
    return;  // Crash here = sleep replays!
}

// GOOD: Flush before returning
entry.dirty = true;
await flush(storage, driver);
return;  // Safe - completion persisted
```

## Replay Behavior

When a workflow resumes after a crash:

1. `loadStorage()` reads all persisted state from KV
2. Workflow code re-executes from the beginning
3. For each operation:
   - If entry exists in history → return cached result (replay)
   - If entry missing → execute operation (forward progress)

This is why flushing is critical: **missing entries mean operations replay**.

## Testing Flush Behavior

To verify flush behavior:

1. Execute workflow until target operation
2. Simulate crash (evict workflow)
3. Resume workflow
4. Verify operation didn't replay (check step execution counts, side effects)

## Performance Considerations

Each `flush()` is a KV batch write. To minimize flushes:

- Use ephemeral steps for non-critical operations
- Batch multiple state changes before flush when safe
- Consider `commitInterval` for batching (TODO: not yet implemented)

However, **never skip flush for durability** - correctness > performance.

## Debugging Tips

1. **HistoryDivergedError**: Usually means an entry wasn't flushed and is missing on replay
2. **Duplicate execution**: Step ran twice = entry wasn't persisted before crash
3. **Wrong deadline**: Listen timeout entry wasn't flushed before yield

Enable debug logging to trace flush operations:
```typescript
// In driver implementation
async batch(writes: KVWrite[]): Promise<void> {
    console.log(`Flushing ${writes.length} entries:`, writes.map(w => w.key));
    // ... actual write
}
```
