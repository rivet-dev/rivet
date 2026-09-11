# Svelte adapter

- Authentication and shared-client ownership belong to the consuming application; do not add application-specific session or authorization behavior.
- Use a package-local framework bridge until the shared framework package forwards dynamic connection parameters; standalone installs must not require root patches.
- Construction is inert; only a mounted browser lifecycle may acquire a connection, and every mount reference must be released independently.
- Preserve `preloadActor` as the deprecated alias of `warmUp`, and preserve reusable raw-connection disposal.
- Inspector snapshots must use opaque IDs and must never expose raw framework hashes or connection parameters.
- Validate from this directory with `pnpm test`, `pnpm check-types`, and `pnpm build`; verify the packed package against public dependencies when updating the SDK contract.
