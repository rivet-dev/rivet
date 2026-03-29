# Cron Scheduling Spec

Pluggable cron scheduling for agentOS. Schedule recurring or one-shot jobs (agent sessions, commands, arbitrary callbacks) with a swappable driver so the scheduling mechanism can be replaced by an external workflow engine.

## Motivation

agentOS has no way to schedule recurring or delayed work. Running an agent every hour, retrying a failed task after 5 minutes, or triggering a command on a schedule requires building custom orchestration on top of the API. This is common enough to belong in the core.

The key constraint: agentOS runs in many environments. In a simple Node.js process, `setTimeout` is fine. In a workflow engine (Temporal, Inngest, AWS Step Functions), timers must be durable and managed externally. The scheduling mechanism must be pluggable.

## Architecture

```
AgentOs
  └── CronManager              (owns job registry, executes actions)
        └── ScheduleDriver      (pluggable: when does the callback fire?)
              ├── TimerScheduleDriver    (default, setTimeout-based)
              ├── TemporalScheduleDriver (external, durable)
              └── ...
```

The driver only controls *when* the callback fires. The CronManager handles *what* happens (creating sessions, running commands, etc.).

## ScheduleDriver Interface

```typescript
interface ScheduleDriver {
  /**
   * Schedule a callback to fire on a cron expression or at a specific time.
   * Returns a handle that can be used to cancel.
   */
  schedule(entry: ScheduleEntry): ScheduleHandle;

  /**
   * Cancel a previously scheduled entry.
   */
  cancel(handle: ScheduleHandle): void;

  /**
   * Tear down all scheduled work. Called on AgentOs.dispose().
   */
  dispose(): void;
}

interface ScheduleEntry {
  /** Unique ID for this job */
  id: string;
  /** Standard 5-field cron expression ("*/5 * * * *") OR ISO 8601 timestamp for one-shot */
  schedule: string;
  /** Called when the schedule fires */
  callback: () => void | Promise<void>;
}

interface ScheduleHandle {
  id: string;
}
```

This is the only interface a custom driver needs to implement. The driver has no knowledge of agentOS, sessions, or commands -- it just calls a callback on a schedule.

## Default Driver: TimerScheduleDriver

Uses `setTimeout` / `long-timeout` to schedule the next tick. For cron expressions, it parses the expression, computes the next fire time, and sets a single timeout. After each fire, it recomputes and schedules the next one.

```typescript
class TimerScheduleDriver implements ScheduleDriver {
  private timers = new Map<string, ReturnType<typeof setTimeout>>();
  private entries = new Map<string, ScheduleEntry>();

  schedule(entry: ScheduleEntry): ScheduleHandle {
    this.entries.set(entry.id, entry);
    this.scheduleNext(entry);
    return { id: entry.id };
  }

  private scheduleNext(entry: ScheduleEntry): void {
    const next = isCronExpression(entry.schedule)
      ? computeNextCronTime(entry.schedule)
      : new Date(entry.schedule);  // ISO 8601 one-shot
    const delay = Math.max(0, next.getTime() - Date.now());

    const timer = longSetTimeout(async () => {
      this.timers.delete(entry.id);
      await entry.callback();
      // Reschedule for recurring cron, not for one-shot
      if (isCronExpression(entry.schedule) && this.entries.has(entry.id)) {
        this.scheduleNext(entry);
      } else {
        this.entries.delete(entry.id);
      }
    }, delay);

    this.timers.set(entry.id, timer);
  }

  cancel(handle: ScheduleHandle): void {
    const timer = this.timers.get(handle.id);
    if (timer) {
      clearLongTimeout(timer);
      this.timers.delete(handle.id);
    }
    this.entries.delete(handle.id);
  }

  dispose(): void {
    for (const timer of this.timers.values()) clearLongTimeout(timer);
    this.timers.clear();
    this.entries.clear();
  }
}
```

**Why long-timeout?** Standard `setTimeout` overflows at ~24.8 days (2^31 ms). `long-timeout` chains shorter timeouts to handle arbitrary delays. Only needed by this driver -- external drivers manage their own timers.

## CronManager

Internal class that bridges ScheduleDriver and AgentOs. Not exported directly -- accessed through AgentOs methods.

```typescript
class CronManager {
  private jobs = new Map<string, CronJobState>();
  private driver: ScheduleDriver;
  private vm: AgentOs;

  constructor(vm: AgentOs, driver: ScheduleDriver) {
    this.driver = driver;
    this.vm = vm;
  }

  schedule(options: CronJobOptions): CronJob {
    const id = options.id ?? crypto.randomUUID();
    const state: CronJobState = {
      id,
      schedule: options.schedule,
      action: options.action,
      overlap: options.overlap ?? 'allow',
      lastRun: undefined,
      nextRun: computeNextTime(options.schedule),
      runCount: 0,
      running: false,
    };

    const handle = this.driver.schedule({
      id,
      schedule: options.schedule,
      callback: () => this.executeJob(state),
    });

    state.handle = handle;
    this.jobs.set(id, state);
    return { id, cancel: () => this.cancel(id) };
  }

  private async executeJob(state: CronJobState): Promise<void> {
    // Overlap policy
    if (state.running && state.overlap === 'skip') return;
    if (state.running && state.overlap === 'queue') {
      state.queued = true;
      return;
    }

    state.running = true;
    state.lastRun = new Date();
    state.runCount++;

    try {
      await this.runAction(state.action);
      this.emit('cron:complete', { jobId: state.id, time: new Date(), durationMs: Date.now() - state.lastRun.getTime() });
    } catch (error) {
      this.emit('cron:error', { jobId: state.id, time: new Date(), error: error as Error });
    } finally {
      state.running = false;
      state.nextRun = isCronExpression(state.schedule) ? computeNextTime(state.schedule) : undefined;
      // Process queued execution
      if (state.queued) {
        state.queued = false;
        this.executeJob(state);
      }
    }
  }

  private async runAction(action: CronAction): Promise<void> {
    switch (action.type) {
      case 'session': {
        const session = await this.vm.createSession(action.agentType, action.options);
        try {
          await session.prompt(action.prompt);
        } finally {
          await session.close();
        }
        break;
      }
      case 'exec': {
        await this.vm.exec(action.command, { args: action.args });
        break;
      }
      case 'callback': {
        await action.fn();
        break;
      }
    }
  }

  cancel(id: string): void { ... }
  list(): CronJobInfo[] { ... }
  dispose(): void { ... }
}
```

## AgentOs API

### Options

```typescript
interface AgentOsOptions {
  // ...existing
  scheduleDriver?: ScheduleDriver;  // default: new TimerScheduleDriver()
}
```

### Methods

```typescript
class AgentOs {
  /**
   * Schedule a recurring or one-shot job.
   */
  scheduleCron(options: CronJobOptions): CronJob;

  /**
   * List all registered cron jobs.
   */
  listCronJobs(): CronJobInfo[];

  /**
   * Cancel a cron job by ID.
   */
  cancelCronJob(id: string): void;

  /**
   * Subscribe to cron lifecycle events.
   */
  onCronEvent(handler: CronEventHandler): void;
}
```

### Types

```typescript
interface CronJobOptions {
  /** Optional ID. Auto-generated UUID if omitted. */
  id?: string;
  /** Standard 5-field cron expression ("*/5 * * * *") or ISO 8601 timestamp for one-shot */
  schedule: string;
  /** What to do when it fires */
  action: CronAction;
  /** What to do if previous execution is still running. Default: 'allow' */
  overlap?: 'allow' | 'skip' | 'queue';
}

type CronAction =
  | { type: 'session'; agentType: AgentType; prompt: string; options?: CreateSessionOptions }
  | { type: 'exec'; command: string; args?: string[] }
  | { type: 'callback'; fn: () => void | Promise<void> };

interface CronJob {
  id: string;
  cancel(): void;
}

interface CronJobInfo {
  id: string;
  schedule: string;
  action: CronAction;
  overlap: 'allow' | 'skip' | 'queue';
  lastRun?: Date;
  nextRun?: Date;
  runCount: number;
  running: boolean;
}

type CronEvent =
  | { type: 'cron:fire'; jobId: string; time: Date }
  | { type: 'cron:complete'; jobId: string; time: Date; durationMs: number }
  | { type: 'cron:error'; jobId: string; time: Date; error: Error };

type CronEventHandler = (event: CronEvent) => void;
```

## Why This Belongs in agentOS, Not secure-exec

Per the CLAUDE.md rule: "Prefer implementing in secure-exec when a feature is fundamentally an OS-level concern." Cron scheduling is orchestration logic, not an OS primitive. The kernel manages processes, filesystems, and networking. Scheduling when to create sessions or run commands is application-layer coordination. The schedule driver might not even be a timer -- it could be an external workflow engine that has no concept of the kernel. This belongs in agentOS.

## Example: Workflow Engine Driver

For Temporal, Inngest, or similar:

```typescript
class TemporalScheduleDriver implements ScheduleDriver {
  private callbacks = new Map<string, () => void | Promise<void>>();

  constructor(private client: TemporalClient, private signalTarget: WorkflowHandle) {}

  schedule(entry: ScheduleEntry): ScheduleHandle {
    this.callbacks.set(entry.id, entry.callback);
    // Create a Temporal schedule that signals this workflow when it fires
    this.client.schedule.create({
      scheduleId: entry.id,
      spec: { cronExpressions: [entry.schedule] },
      action: {
        type: 'startWorkflow',
        workflowType: 'cronTick',
        args: [entry.id],
      },
    });
    return { id: entry.id };
  }

  // When Temporal fires, it signals this process which calls:
  async handleTick(jobId: string): Promise<void> {
    const cb = this.callbacks.get(jobId);
    if (cb) await cb();
  }

  cancel(handle: ScheduleHandle): void {
    this.client.schedule.delete(handle.id);
    this.callbacks.delete(handle.id);
  }

  dispose(): void {
    for (const id of this.callbacks.keys()) {
      this.client.schedule.delete(id);
    }
    this.callbacks.clear();
  }
}
```

## File Plan

| File | What |
|------|------|
| `packages/core/src/cron/schedule-driver.ts` | `ScheduleDriver` interface, `ScheduleEntry`, `ScheduleHandle` types |
| `packages/core/src/cron/timer-driver.ts` | `TimerScheduleDriver` default implementation |
| `packages/core/src/cron/cron-manager.ts` | Internal `CronManager` class -- job registry, action execution, events |
| `packages/core/src/cron/types.ts` | `CronJobOptions`, `CronAction`, `CronJobInfo`, `CronJob`, `CronEvent`, `CronEventHandler` |
| `packages/core/src/cron/index.ts` | Barrel export |
| `packages/core/src/agent-os.ts` | Add `scheduleCron`, `listCronJobs`, `cancelCronJob`, `onCronEvent`; wire `scheduleDriver` option; call `cronManager.dispose()` in `dispose()` |
| `packages/core/src/index.ts` | Re-export cron types |

## Dependencies

- **`croner`** -- Parse and evaluate cron expressions. Small (~4KB), zero deps, supports standard 5-field and 6-field (seconds) syntax. Preferred over `cron-parser` for simpler API.
- **`long-timeout`** -- setTimeout wrapper for delays >2^31ms. Only used by `TimerScheduleDriver`.

## Not In Scope

- **Persistence / durability**: Jobs live in memory. If the process dies, they're gone. A persistent driver (backed by SQLite, Redis, etc.) is a custom `ScheduleDriver`, not a core feature.
- **Distributed locking**: Single-process only. Multi-instance dedup is the workflow engine driver's responsibility.
- **Timezone support**: Cron expressions evaluated in host local time (matching standard cron). Per-job timezone can be added later.
- **Job dependencies / DAGs**: Each job is independent. If you need "run B after A completes", use a callback action that schedules B.

## Testing Strategy

### Unit Tests

- **TimerScheduleDriver**: schedule fires callback after computed delay, cancel prevents fire, dispose clears all timers, one-shot doesn't reschedule, cron reschedules after fire
- **CronManager**: schedule/cancel/list lifecycle, action execution for each type (session, exec, callback), overlap policies (allow runs concurrent, skip drops, queue waits), error in action emits cron:error event, cron:complete emits with duration

### Integration Tests

- **AgentOs.scheduleCron**: schedule exec job with short cron, verify it fires via side effect (file write), cancel stops further fires
- **Session action**: schedule session action with mock LLM, verify session created and prompt sent
- **Custom driver**: pass custom ScheduleDriver to AgentOs.create, verify it receives schedule/cancel calls instead of default timer

Use fake timers (vitest `vi.useFakeTimers()`) for unit tests to avoid real delays. Integration tests use real timers with short intervals.
