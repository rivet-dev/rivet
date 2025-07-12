# Hub Architecture

## Overview

The Hub is a React/TypeScript dashboard application built with:
- **TanStack Router** - File-based routing
- **TanStack Query** - Server state management
- **Jotai** - Complex client state
- **React Context** - Global app state
- **Tailwind CSS** - Styling

## Directory Structure

```
src/
├── domains/          # Feature domains
│   └── {domain}/
│       ├── components/   # Domain-specific components
│       ├── queries/      # Query options & mutations
│       ├── data/         # Context providers
│       └── forms/        # Form schemas
├── routes/          # Page components (file-based)
├── components/      # Shared UI components
├── queries/         # Global query setup
├── hooks/           # Custom React hooks
├── lib/             # Utilities
└── app.tsx          # Root setup
```

## State Management

### Server State (TanStack Query)
```typescript
// Query pattern: src/domains/{domain}/queries/query-options.ts
export const actorLogsQueryOptions = ({ projectNameId, environmentNameId, actorId }) => 
  queryOptions({
    queryKey: ["project", projectNameId, "environment", environmentNameId, "actor", actorId, "logs"],
    queryFn: async ({ signal }) => rivetClient.actors.logs.get(...),
    meta: { watch: true }, // Enable real-time updates
  });
```

### Client State (Jotai)
Used for complex, modular state (e.g., actor management):
```typescript
// Atoms for fine-grained reactivity
export const currentActorIdAtom = atom<string | undefined>(undefined);
export const actorsAtom = atom<Actor[]>([]);
```

### Global State (React Context)
```typescript
// Context for app-wide concerns
AuthContext      // User authentication
ProjectContext   // Current project
EnvironmentContext // Current environment
```

## Query Patterns

### Standard Query
```typescript
const { data } = useQuery(projectQueryOptions(projectId));
```

### Infinite Query
```typescript
const { data, fetchNextPage } = useInfiniteQuery(
  projectActorsQueryOptions({ projectNameId, environmentNameId })
);
```

### Mutations
```typescript
const mutation = useMutation({
  mutationFn: (data) => rivetClient.actors.destroy(data),
  onSuccess: () => queryClient.invalidateQueries(...)
});
```

### Real-time Updates
Queries with `watchIndex` parameter automatically refetch when server data changes:
```typescript
queryFn: ({ meta }) => api.call({ 
  watchIndex: getMetaWatchIndex(meta) 
})
```

## Key Conventions

### Query Keys
Hierarchical structure matching API resources:
```typescript
["project", projectId, "environment", envId, "actor", actorId, "logs"]
```

### File Naming
- Query options: `query-options.ts`
- Mutations: `mutations.ts`
- Context: `{resource}-context.tsx`
- Components: `{feature}-{component}.tsx`

### Import Aliases
```typescript
@/domains     // Domain logic
@/components  // Shared components
@/queries     // Query utilities
@/hooks       // Custom hooks
@/lib         // Utilities
```

## API Integration

- **rivetClient**: Main API client (`@rivet-gg/api-full`)
- **rivetEeClient**: Enterprise API (`@rivet-gg/api-ee`)
- Auto token refresh on expiration
- Request deduplication & caching

## Example: Adding a New Feature

1. **Create query options**:
   ```typescript
   // src/domains/project/queries/feature/query-options.ts
   export const featureQueryOptions = (id: string) => queryOptions({
     queryKey: ["feature", id],
     queryFn: () => rivetClient.feature.get(id),
   });
   ```

2. **Create mutations**:
   ```typescript
   // src/domains/project/queries/feature/mutations.ts
   export const useCreateFeatureMutation = () => useMutation({
     mutationFn: (data) => rivetClient.feature.create(data),
   });
   ```

3. **Create route**:
   ```typescript
   // src/routes/.../feature.tsx
   export const Route = createFileRoute('...')({
     component: FeaturePage,
   });
   ```

4. **Use in component**:
   ```typescript
   function FeaturePage() {
     const { data } = useQuery(featureQueryOptions(id));
     const mutation = useCreateFeatureMutation();
     // ...
   }
   ```