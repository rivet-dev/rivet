# RivetKit Framework Base

_Library to build and scale stateful workloads_

[Learn More →](https://github.com/rivet-dev/rivetkit)

[Discord](https://rivet.dev/discord) — [Documentation](https://rivetkit.org) — [Issues](https://github.com/rivet-dev/rivetkit/issues)

## Lifecycle

### Mount

```
1. useActor(opts) called in React component

2. getOrCreateActor(opts)
   - hash opts to get key
   - sync opts to store (create or update actor entry)
   - if not in cache:
       - create Derived (subscribes to store)
       - create Effect (handles connection logic)
       - add to cache with refCount=0
   - return { mount, state }

3. useEffect runs mount()
   - cancel any pending cleanup timeout
   - refCount++
   - if refCount == 1:
       - mount derived and effect
       - if enabled and idle: call create() directly
         (Effect only runs on state changes, not on mount)

4. Effect triggers (on state changes)
   - if disabled and connected: dispose connection, reset to idle
   - if enabled and idle: call create()

5. create()
   - set connStatus = Connecting
   - handle = client.getOrCreate(name, key)
   - connection = handle.connect()
   - subscribe to connection status/error events
   - store handle and connection in store

6. Connection established
   - connStatus updates, Derived updates, React re-renders
```

### Unmount

```
1. Component unmounts
2. useEffect cleanup runs
3. refCount--
4. if refCount == 0: setTimeout(cleanup, 0)
5. When timeout fires:
   - if refCount > 0: skip (was remounted)
   - else: dispose connection, remove from store/cache
```

### React Strict Mode

Why `setTimeout` matters:

```
- render
- mount: refCount = 1
- unmount: refCount = 0, schedule timeout
- remount: refCount = 1, cancel timeout
- timeout fires: refCount > 0, cleanup skipped
```

### Shared Actor

Two components using the same actor opts:

```
- Component A mounts: refCount = 1, connection created
- Component B mounts: refCount = 2, reuses connection
- Component A unmounts: refCount = 1, no cleanup
- Component B unmounts: refCount = 0, cleanup scheduled
- Timeout fires: connection disposed, removed from cache
```

## License

Apache 2.0