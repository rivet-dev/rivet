# Guard Routing & Retry Behavior

## Guard Core - Retry Behavior

The retry mechanism enables optimistic routing with cache invalidation:

- **Fast path with cached routes**: Guard uses cached routing information to avoid expensive database lookups on every request, providing low-latency routing decisions
- **Graceful cache invalidation**: When actors are stopped, destroyed, or moved to different runners, the cache becomes stale. Rather than proactively invalidating cache entries (which would require complex coordination), any failure response signals that the cached route is invalid
- **Fresh service discovery on retry**: Retry attempts ignore the cache and perform fresh database lookups to discover current actor locations, ensuring requests reach the correct destination

This approach optimizes for the common case (actors are running and routes are valid) while gracefully handling the uncommon case (actors have moved/stopped) without sacrificing performance

### Retry Flow

**Initial Request (Attempt 1)**:

1. Check route cache for target location
2. If cached route exists, send request to cached target
3. If request succeeds → return response to client
4. If request fails with retry-able error → proceed to retry

**Retry Attempts (Attempts 2-N)**:

1. Wait for exponential backoff delay (100ms × 2^(attempt-2))
2. **Ignore cache** and perform fresh database lookup for target location
3. Send request to newly discovered target
4. If request succeeds → return response to client
5. If request fails and max attempts not reached → repeat retry flow
6. If max attempts exceeded → return `502 Bad Gateway` to client

**Configuration**:

- **Exponential backoff**: Starting interval 100ms, doubles each attempt (100ms, 200ms, 400ms...)
- **Maximum attempts**: Default 3 total attempts
- **Retry triggers**: TCP connection errors OR `503 Service Unavailable` with `x-rivet-error` header

### Expected Service Response for Retries

Services that want to trigger guard retries must respond with:

- `503 Service Unavailable` status code
- `x-rivet-error: <error>` header

## Guard Router - Routing Logic

### Routing Priority

Requests are routed in the following priority order:

#### 1. Target-Based Routing (Header: `x-rivet-target`)

When the `x-rivet-target` header is present, routes to specific service types:

**Actor Services** (`x-rivet-target: actor`):
- **Required headers**: 
  - `x-rivet-actor: <actor_id>` - UUID of the specific actor instance
- **Optional headers**:
  - `x-rivet-addr: <address>` - Direct address override for actor location
- **Behavior**: Routes to the specific actor instance, with cross-datacenter routing if the actor resides in a different DC

**Runner WebSocket** (`x-rivet-target: runner-ws`):
- **Purpose**: Routes WebSocket connections to the Pegboard runner service
- **Target**: Routes to the configured Pegboard service (`pegboard.lan_host:pegboard.port`)
- **Use case**: WebSocket connections between runners and the orchestration system

#### 2. API Routing (No target header)

When no `x-rivet-target` header is present:
- **Target**: Routes to the public API service (`api_public.lan_host:api_public.port`)
- **Behavior**: Standard HTTP API requests for general application functionality
- **Path preservation**: The original request path is preserved in the upstream request

#### 3. Fallback

Returns `404 Not Found` if no routing rules match.

