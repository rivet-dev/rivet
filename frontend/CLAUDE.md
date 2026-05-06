# Frontend

- When building new UI components, ship a Ladle story alongside the component (`*.stories.tsx` next to the source). Cover the meaningful states (empty, loading, error, success, edge cases) so the component can be reviewed and iterated on in isolation without booting the whole dashboard. Run with `pnpm dev:ladle` from `frontend/`. Existing example: [src/app/runner-pool-error-popover.stories.tsx](src/app/runner-pool-error-popover.stories.tsx).
