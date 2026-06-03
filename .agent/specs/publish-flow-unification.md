# Publish Flow Unification

## Summary

Merge `.github/workflows/release.yaml` and `.github/workflows/preview-publish.yaml` into a single `publish.yaml` workflow. Collapse `scripts/release/` and `scripts/preview-publish/` into a single `scripts/publish/` package organized as `src/lib`, `src/ci`, `src/local`. Drop `reuse_engine_version`, the phase-based release script, the devtools-to-R2 upload, and the CI-side invocation of the release orchestrator script.

The same build, publish, R2 upload, and Docker manifest steps run for preview PRs, main pushes, and release cuts. The only release-specific tail is: npm dist-tag selection, R2/Docker retagging to the version name, git tag, and GitHub release.

## Goals

1. One CI code path for preview and release. The only divergence is a four-step release tail gated by `github.event_name == 'workflow_dispatch'`.
2. One script package with a clean three-folder structure: `src/lib` (shared logic), `src/ci` (GitHub Actions entrypoints), `src/local` (human release cutter).
3. Delete all reuse-engine-version plumbing.
4. Delete all CI-side invocation of the release orchestrator script. The workflow calls narrow, single-purpose subcommands directly.
5. Each subcommand is a pure function of its CLI args and a single `PublishContext`, making steps independently testable and locally reproducible.

## Non-goals

- Changing what gets published (which packages, which platforms).
- Changing the build Dockerfiles.
- Changing base image tags or depot configuration.
- Adding new features (test matrix, canary channels, staged rollouts).
- Rewriting the Docker image build pipeline (`docker/engine/Dockerfile` → `rivetdev/engine:{slim,full}`); that job stays as-is.
- Workspace-package splitting. Everything stays in one package.

## Current state (reference)

### Files today

```
scripts/release/
  main.ts                 # 630 lines — phase-based orchestrator, local + CI
  sdk.ts                  # 267 lines — serial npm publish with hardcoded platform loops
  build-artifacts.ts      # 37 lines — devtools upload to R2
  promote-artifacts.ts    # 88 lines — S3 commit → version copy with reuse branches
  docker.ts               # 107 lines — multi-arch manifest with reuse branches
  git.ts                  # 85 lines — validate, tag, gh release
  update_version.ts       # 106 lines — regex rewrites across many glob paths
  utils.ts                # 200 lines — R2 client + reuse helpers
  static/install.sh       # R2-hosted install script template
  static/install.ps1

scripts/preview-publish/
  discover-packages.ts    # 126 lines — single source of truth for publishable pkgs
  bump-versions.ts        # 143 lines — rewrite versions + inject optionalDependencies
  publish-all.ts          # 246 lines — parallel npm publish with retries

.github/workflows/
  release.yaml            # 323 lines — setup → binaries + docker → complete
  preview-publish.yaml    # 413 lines — build matrix → publish
```

Total script lines today: ~2035.

### Key problems

1. `sdk.ts` and `publish-all.ts` do the same thing, differently. `sdk.ts` is serial with hardcoded platform loops. `publish-all.ts` is parallel with generic discovery. Both have their own exclusion lists.
2. `update_version.ts` globs for `package.json` files and rewrites them with regex. `discover-packages.ts` independently walks similar paths. They drift.
3. `main.ts` runs in three phases (`setup-local`, `setup-ci`, `complete-ci`) gated by `--phase` flags. CI invokes the same script with different phases. Adding a step requires touching the step list, phase map, and one of three CI jobs.
4. `reuse_engine_version` adds branches in `main.ts`, `promote-artifacts.ts`, `docker.ts`, plus the `validateReuseVersion` function. The user has decided this is dead — engine builds are fast enough.
5. Preview doesn't push Docker images or upload R2 artifacts. Release does. Users can't test an engine PR via Docker or curl.
6. Release ships engine three ways (R2 binary, Docker image, npm platform package). Preview ships one way (npm). Divergent test surface.

## Target architecture

### Directory structure

```
scripts/publish/
  package.json
  tsconfig.json
  .eslintrc.cjs                 # enforces src/ci ↮ src/local ↮ src/lib boundaries
  src/
    lib/
      context.ts                # PublishContext type + resolver
      packages.ts               # discoverPackages + exclusion lists (single source)
      version.ts                # resolveVersion, bumpPackageJsons, updateSourceFiles
      npm.ts                    # publishAll (parallel + retries)
      r2.ts                     # S3 client + upload + copy + list
      docker.ts                 # createMultiArchManifest + retagManifest
      git.ts                    # validateClean + tagAndPush + createGhRelease
      logger.ts                 # prefixed console wrapper
    ci/
      bin.ts                    # commander root — all CI subcommands
    local/
      cut-release.ts            # linear orchestrator for the human release flow
```

Invariants:

- `src/lib/*` imports only from `src/lib/*`.
- `src/ci/*` and `src/local/*` import only from `src/lib/*`.
- `src/ci/*` never imports from `src/local/*` and vice versa.
- `src/local/cut-release.ts` is never invoked by CI.
- `src/lib/version.ts#updateSourceFiles` is only called by `src/local/cut-release.ts`.

These are enforced by ESLint `no-restricted-imports`.

### PublishContext

The shape that flows through every subcommand:

```ts
// src/lib/context.ts
export type Trigger = "pr" | "main" | "release";

export interface PublishContext {
  trigger: Trigger;
  version: string;           // resolved, never null (e.g. "2.5.0" or "2.5.0-pr.4600.abc1234")
  npmTag: string;            // "pr-N" | "main" | "latest" | "rc"
  sha: string;               // short sha (7 chars)
  latest: boolean;           // only meaningful when trigger === "release"
  prNumber?: number;
  repoRoot: string;
}

export async function resolveContext(overrides?: Partial<PublishContext>): Promise<PublishContext>;
```

`resolveContext()` reads GitHub Actions env vars (`GITHUB_EVENT_NAME`, `GITHUB_SHA`, `GITHUB_EVENT_PATH`, workflow inputs), merges any explicit overrides (used by `cut-release.ts`), and returns a fully-resolved context. Memoized per process via a module-level cache.

The `ci context-output` subcommand calls `resolveContext()`, writes every field to `$GITHUB_OUTPUT`, and exits. Subsequent workflow steps read fields via `${{ steps.ctx.outputs.* }}` instead of recomputing.

### Subcommands exposed by `src/ci/bin.ts`

```
pnpm --filter=@rivet/publish ci context-output
pnpm --filter=@rivet/publish ci bump-versions
pnpm --filter=@rivet/publish ci publish-npm --parallel 16 --retries 3
pnpm --filter=@rivet/publish ci upload-r2 --source <dir>
pnpm --filter=@rivet/publish ci copy-r2       # release-only (uses ctx.version + ctx.latest)
pnpm --filter=@rivet/publish ci docker-manifest
pnpm --filter=@rivet/publish ci docker-retag   # release-only
pnpm --filter=@rivet/publish ci git-tag        # release-only
pnpm --filter=@rivet/publish ci gh-release     # release-only
pnpm --filter=@rivet/publish ci comment-pr     # preview PR only
```

Every subcommand accepts `ctx` derived from `resolveContext()`. No subcommand orchestrates another.

### Local entrypoint

```
pnpm --filter=@rivet/publish release --version 2.5.0 [--no-latest] [--major|--minor|--patch]
```

`src/local/cut-release.ts` is linear:

```
resolveContext({ trigger: "release", version })
validateClean()
confirmPrompt()
updateSourceFiles()         # Cargo.toml, examples, protocol version constants
bumpPackageJsons()          # all package.jsons via packages.ts discovery
runFernGen()                # ./scripts/fern/gen.sh
runLocalTypecheck()         # fail-fast type check + cargo check
commitAndPush()             # git add . + commit + push (or gt submit)
triggerWorkflow()           # gh workflow run publish.yaml -f version=...
```

No phases, no `shouldRunStep`, no `--only-steps`. Debugging = comment out a line.

### Merged workflow

```
.github/workflows/publish.yaml
  on:
    pull_request:
    push:
      branches: [main]
    workflow_dispatch:
      inputs:
        version: { required: true, type: string }
        latest:  { required: true, type: boolean, default: true }

  jobs:
    context:
      runs-on: ubuntu-24.04
      outputs:
        trigger:  ${{ steps.ctx.outputs.trigger }}
        version:  ${{ steps.ctx.outputs.version }}
        npm_tag:  ${{ steps.ctx.outputs.npm_tag }}
        sha:      ${{ steps.ctx.outputs.sha }}
        latest:   ${{ steps.ctx.outputs.latest }}
      steps:
        - uses: actions/checkout@v4
        - run: corepack enable
        - uses: actions/setup-node@v4
          with: { node-version: '22', cache: pnpm }
        - run: pnpm install --frozen-lockfile --filter=@rivet/publish
        - id: ctx
          run: pnpm --filter=@rivet/publish ci context-output

    build:
      needs: [context]
      name: "Build ${{ matrix.name }}"
      strategy:
        fail-fast: false
        matrix:
          include:
            # Same 10-entry matrix as current preview-publish.yaml.
      runs-on: depot-ubuntu-24.04-8
      permissions:
        contents: read
        id-token: write
        packages: read
      env:
        BASE_TAG: 0e33ceb98
        DEPOT_PROJECT_ID: 1rcpv5rn8n
      steps:
        - uses: actions/checkout@v4
          with: { lfs: true }
        - uses: depot/setup-action@v1
        - name: Log in to ghcr.io
          run: echo "${{ secrets.GITHUB_TOKEN }}" | docker login ghcr.io -u ${{ github.actor }} --password-stdin
        - name: Compute build mode
          id: mode
          run: |
            if [ "${{ needs.context.outputs.trigger }}" = "release" ]; then
              echo "build_mode=release"    >> $GITHUB_OUTPUT
              echo "build_frontend=true"   >> $GITHUB_OUTPUT
            else
              echo "build_mode=debug"      >> $GITHUB_OUTPUT
              echo "build_frontend=false"  >> $GITHUB_OUTPUT
            fi
        - name: Build via depot
          env:
            DEPOT_TOKEN: ${{ secrets.DEPOT_TOKEN }}
          run: |
            depot build \
              --project ${{ env.DEPOT_PROJECT_ID }} \
              --secret id=DEPOT_TOKEN,env=DEPOT_TOKEN \
              --build-arg BASE_TAG=${{ env.BASE_TAG }} \
              --build-arg BUILD_TARGET=${{ matrix.build_target }} \
              --build-arg BUILD_MODE=${{ steps.mode.outputs.build_mode }} \
              --build-arg BUILD_FRONTEND=${{ steps.mode.outputs.build_frontend }} \
              -f ${{ matrix.docker }} \
              -t builder-${{ matrix.build_target }}-${{ matrix.platform }} \
              --load .
            CONTAINER_ID=$(docker create builder-${{ matrix.build_target }}-${{ matrix.platform }})
            mkdir -p artifacts
            docker cp "$CONTAINER_ID:/artifacts/${{ matrix.artifact }}" artifacts/
            docker rm "$CONTAINER_ID"
        - uses: actions/upload-artifact@v4
          with:
            name: ${{ matrix.upload_prefix }}-${{ matrix.platform }}
            path: artifacts/${{ matrix.artifact }}
            if-no-files-found: error

    docker-images:
      needs: [context]
      name: "Docker ${{ matrix.arch_suffix }}"
      strategy:
        matrix:
          include:
            - platform: linux/amd64
              runner: depot-ubuntu-24.04-8
              arch_suffix: -amd64
            - platform: linux/arm64
              runner: depot-ubuntu-24.04-arm-8
              arch_suffix: -arm64
      runs-on: ${{ matrix.runner }}
      steps:
        - uses: actions/checkout@v4
          with: { lfs: true }
        - uses: ./.github/actions/docker-setup
          with:
            docker_username: ${{ secrets.DOCKER_CI_USERNAME }}
            docker_password: ${{ secrets.DOCKER_CI_ACCESS_TOKEN }}
            github_token:    ${{ secrets.GITHUB_TOKEN }}
        - name: Compute build mode
          id: mode
          run: |
            if [ "${{ needs.context.outputs.trigger }}" = "release" ]; then
              echo "cargo_build_mode=release" >> $GITHUB_OUTPUT
            else
              echo "cargo_build_mode=debug"   >> $GITHUB_OUTPUT
            fi
        - uses: docker/build-push-action@v4
          with:
            context: .
            push: true
            tags: rivetdev/engine:full-${{ needs.context.outputs.sha }}${{ matrix.arch_suffix }}
            file: docker/engine/Dockerfile
            target: engine-full
            platforms: ${{ matrix.platform }}
            build-args: |
              BUILD_FRONTEND=true
              CARGO_BUILD_MODE=${{ steps.mode.outputs.cargo_build_mode }}
        - uses: docker/build-push-action@v4
          with:
            context: .
            push: true
            tags: rivetdev/engine:slim-${{ needs.context.outputs.sha }}${{ matrix.arch_suffix }}
            file: docker/engine/Dockerfile
            target: engine-slim
            platforms: ${{ matrix.platform }}
            build-args: |
              BUILD_FRONTEND=true
              CARGO_BUILD_MODE=${{ steps.mode.outputs.cargo_build_mode }}

    publish:
      needs: [context, build, docker-images]
      if: ${{ !cancelled() && needs.build.result != 'failure' && needs.docker-images.result != 'failure' }}
      runs-on: depot-ubuntu-24.04-8
      permissions:
        contents: write          # git tag + gh release (release only)
        id-token: write
        pull-requests: write     # PR comment
      env:
        NODE_AUTH_TOKEN:              ${{ secrets.NPM_TOKEN }}
        R2_RELEASES_ACCESS_KEY_ID:    ${{ secrets.R2_RELEASES_ACCESS_KEY_ID }}
        R2_RELEASES_SECRET_ACCESS_KEY: ${{ secrets.R2_RELEASES_SECRET_ACCESS_KEY }}
        GH_TOKEN:                     ${{ secrets.GITHUB_TOKEN }}
      steps:
        - uses: actions/checkout@v4
        - run: corepack enable
        - uses: actions/setup-node@v4
          with: { node-version: '22', registry-url: 'https://registry.npmjs.org', cache: pnpm }
        - run: pnpm install --frozen-lockfile
        - uses: ./.github/actions/docker-setup
          with:
            docker_username: ${{ secrets.DOCKER_CI_USERNAME }}
            docker_password: ${{ secrets.DOCKER_CI_ACCESS_TOKEN }}
            github_token:    ${{ secrets.GITHUB_TOKEN }}

        # ---- shared artifact placement + build ----
        - uses: actions/download-artifact@v4
          with:
            path: native-artifacts
            pattern: native-*
            merge-multiple: true
        - uses: actions/download-artifact@v4
          with:
            path: engine-artifacts
            pattern: engine-*
            merge-multiple: true
        - name: Place native binaries
          run: # same inline script as current preview-publish
        - name: Place engine binaries
          run: # same inline script as current preview-publish
        - name: Build TypeScript packages
          run: pnpm build -F rivetkit -F '@rivetkit/*' <exclude filters>
        - name: Pack inspector
          run: npx turbo build:pack-inspector -F rivetkit
        - name: Strip inspector sourcemaps
          run: # same inline script

        # ---- shared publish ----
        - name: Bump versions
          run: pnpm --filter=@rivet/publish ci bump-versions
        - name: Publish npm packages
          run: pnpm --filter=@rivet/publish ci publish-npm --parallel 16 --retries 3
        - name: Upload engine binaries to R2
          run: pnpm --filter=@rivet/publish ci upload-r2 --source engine-artifacts
        - name: Create Docker multi-arch manifests
          run: pnpm --filter=@rivet/publish ci docker-manifest

        # ---- release-only tail ----
        - name: Copy R2 to version path
          if: ${{ needs.context.outputs.trigger == 'release' }}
          run: pnpm --filter=@rivet/publish ci copy-r2
        - name: Retag Docker to version
          if: ${{ needs.context.outputs.trigger == 'release' }}
          run: pnpm --filter=@rivet/publish ci docker-retag
        - name: Git tag
          if: ${{ needs.context.outputs.trigger == 'release' }}
          run: pnpm --filter=@rivet/publish ci git-tag
        - name: GitHub release
          if: ${{ needs.context.outputs.trigger == 'release' }}
          run: pnpm --filter=@rivet/publish ci gh-release

        # ---- preview-only tail ----
        - name: Comment on PR
          if: ${{ needs.context.outputs.trigger == 'pr' }}
          run: pnpm --filter=@rivet/publish ci comment-pr
```

### Per-trigger behavior table

| Step | PR preview | main preview | release |
|---|---|---|---|
| build mode | debug | debug | release |
| build frontend | false | false | true |
| npm version | `{base}-pr.N.sha` | `{base}-main.sha` | `X.Y.Z` |
| npm tag | `pr-N` | `main` | `latest` or `rc` |
| R2 `rivet/{sha}/engine/` upload | ✓ | ✓ | ✓ |
| Docker `{slim,full}-{sha}-{arch}` push | ✓ | ✓ | ✓ |
| Docker `{slim,full}-{sha}` multi-arch manifest | ✓ | ✓ | ✓ |
| R2 `rivet/{version}/engine/` copy | — | — | ✓ |
| R2 `rivet/latest/engine/` copy | — | — | ✓ if latest |
| Docker `{slim,full}-{version}` retag | — | — | ✓ |
| Docker `{slim,full}-latest` retag | — | — | ✓ if latest |
| git tag + force push | — | — | ✓ |
| gh release create | — | — | ✓ |
| PR comment upsert | ✓ | — | — |

## Module responsibilities

### `src/lib/context.ts`

Exports `PublishContext` type and `resolveContext()`. Reads GitHub Actions env and synthesizes:

- `trigger` from `GITHUB_EVENT_NAME` + presence of `workflow_dispatch.inputs.version`
- `version`:
  - PR → `{base}-pr.{GITHUB_EVENT.pull_request.number}.{sha7}`
  - main → `{base}-main.{sha7}`
  - release → `${INPUT_VERSION}`
  - `{base}` is read from `rivetkit-typescript/packages/rivetkit-native/package.json`
- `npmTag`:
  - PR → `pr-{number}`
  - main → `main`
  - release → `latest` unless version contains `-rc.`, then `rc`; unless `--no-latest`, then `next`
- `sha` = `GITHUB_SHA.slice(0, 7)`
- `latest` = workflow_dispatch input `latest`, else false
- `prNumber` = pull_request event payload number
- `repoRoot` = `process.env.GITHUB_WORKSPACE` ?? `git rev-parse --show-toplevel`

Memoized.

### `src/lib/packages.ts`

Exports `discoverPackages(repoRoot): Package[]`, `EXCLUDED: Set<string>`, `META_PACKAGES` array, and `buildMetaPlatformMap()` helper. Single source of truth for:

- Which packages get published (current `discover-packages.ts` logic).
- Exclusion list (`@rivetkit/shared-data`, `@rivetkit/engine-frontend`, `@rivetkit/mcp-hub`, etc.).
- Meta packages and their platform-specific dependency prefixes (`@rivetkit/rivetkit-native` + `@rivetkit/engine-cli`).

### `src/lib/version.ts`

```ts
export async function resolveVersion(opts: { version?: string; major?: boolean; minor?: boolean; patch?: boolean }): Promise<string>;
export async function shouldTagAsLatest(version: string): Promise<boolean>;
export async function bumpPackageJsons(ctx: PublishContext): Promise<void>;
export async function updateSourceFiles(ctx: PublishContext): Promise<void>;
```

- `resolveVersion` = current `getVersionFromArgs` + semver bump logic.
- `shouldTagAsLatest` = current `shouldTagAsLatest`.
- `bumpPackageJsons` = current `bump-versions.ts` logic. Called on every CI run. JSON parse + write (no regex).
- `updateSourceFiles` = non-package-json source files: `Cargo.toml`, `sqlite-native/Cargo.toml`, `examples/**/package.json` (dep spec rewrites), protocol version constants. Only called by `cut-release.ts`.

### `src/lib/npm.ts`

Absorbs `publish-all.ts` + deletes `sdk.ts`. Single export:

```ts
export async function publishAll(ctx: PublishContext, opts: { parallel: number; retries: number }): Promise<Summary>;
```

- Uses `discoverPackages(ctx.repoRoot)` internally.
- Tag comes from `ctx.npmTag`.
- Parallel + retries + already-published detection from current `publish-all.ts`.
- Hard-fails if any package fails for a non-retryable reason.

### `src/lib/r2.ts`

```ts
export async function uploadArtifactsToR2(localDir: string, r2Prefix: string): Promise<void>;
export async function copyR2Path(from: string, to: string): Promise<void>;
export async function uploadInstallScripts(ctx: PublishContext, version: string): Promise<void>;
```

S3 client factory uses env vars directly in CI and falls back to 1Password locally (keeps current behavior). Delete `listReleasesObjects` / `deleteReleasesPath` unless a caller needs them. Keep `copyReleasesPath` workaround for R2 tagging-header bug (document the reason with the current comment).

### `src/lib/docker.ts`

```ts
export async function createMultiArchManifest(name: string, sha: string): Promise<void>;
export async function retagManifest(name: string, from: string, to: string): Promise<void>;
```

No reuse branch. `{slim,full}` loop lives in the caller or as an internal constant. ~30 lines total.

### `src/lib/git.ts`

```ts
export async function validateClean(): Promise<void>;
export async function tagAndPush(version: string): Promise<void>;
export async function createGhRelease(version: string): Promise<void>;
```

### `src/lib/logger.ts`

Tiny prefixed logger:

```ts
export function scoped(prefix: string): { info(msg: string): void; warn(msg: string): void; error(msg: string): void };
```

Consistent prefixes (`[bump]`, `[npm]`, `[r2]`, `[docker]`, `[git]`) for scannable CI logs.

### `src/ci/bin.ts`

Commander root. Each subcommand:

1. Calls `resolveContext()`.
2. Delegates to one `lib/*` function.
3. Prints a one-line summary.
4. Exits 0/1.

`ci context-output` writes every context field to `$GITHUB_OUTPUT` using the format `name=value`.

`ci comment-pr` reads the PR number from the context and upserts the "Preview packages published to npm" comment. Uses `gh api` like the current workflow.

### `src/local/cut-release.ts`

Linear script:

```ts
async function main() {
  const argv = parseArgs();
  const resolvedVersion = await resolveVersion(argv);
  const latest = argv.latest ?? await shouldTagAsLatest(resolvedVersion);
  const ctx = await resolveContext({
    trigger: "release",
    version: resolvedVersion,
    latest,
  });
  await validateClean();
  confirmPrompt(ctx);
  await updateSourceFiles(ctx);
  await bumpPackageJsons(ctx);
  await runFernGen();
  await runLocalTypecheck();
  await commitAndPush(ctx);
  await triggerWorkflow(ctx);
}
```

`runFernGen`, `runLocalTypecheck`, `commitAndPush`, `triggerWorkflow`, `confirmPrompt` live inline in this file (they're each ~10 lines).

## Things that get deleted

| File | Reason |
|---|---|
| `scripts/release/main.ts` | Replaced by `src/local/cut-release.ts` + `src/ci/bin.ts` |
| `scripts/release/sdk.ts` | Replaced by `src/lib/npm.ts` |
| `scripts/release/build-artifacts.ts` | `@rivetkit/devtools` is already published to npm via discovery |
| `scripts/release/promote-artifacts.ts` | Replaced by `src/lib/r2.ts` (reuse branches deleted) |
| `scripts/release/docker.ts` | Replaced by `src/lib/docker.ts` (reuse branches deleted) |
| `scripts/release/git.ts` | Replaced by `src/lib/git.ts` |
| `scripts/release/update_version.ts` | Replaced by `src/lib/version.ts#updateSourceFiles` + `bumpPackageJsons` |
| `scripts/release/utils.ts` | R2 bits moved to `src/lib/r2.ts`, reuse helpers deleted |
| `scripts/release/package.json` + `tsconfig.json` | Replaced by single `scripts/publish/` package |
| `scripts/preview-publish/*` | All three files merged into `src/lib/*` |
| `scripts/release/static/install.{sh,ps1}` | Moved to `scripts/publish/static/` (still uploaded by `uploadInstallScripts`) |
| `.github/workflows/release.yaml` | Merged into `publish.yaml` |
| `.github/workflows/preview-publish.yaml` | Merged into `publish.yaml` |
| `reuse_engine_version` workflow input | User decided this is dead |
| `validateReuseVersion` function | Dead with reuse removal |
| `versionOrCommitToRef`, `fetchGitRef` | Dead with reuse removal |
| `--phase`, `--only-steps`, `--no-validate-git`, `--override-commit`, `--reuse-engine-version` flags | Dead with phase removal |

Total: ~1145 lines removed, ~890 lines added, net ~−255 lines. (Net savings come from removing reuse plumbing, deleting devtools R2 path, and unifying sdk.ts + publish-all.ts.)

## Migration plan

Each step is independently revertable and leaves CI working.

### Step 1: Bootstrap `scripts/publish/`

- Create `scripts/publish/{package.json,tsconfig.json,.eslintrc.cjs}`.
- Create `src/lib/{context.ts,packages.ts,logger.ts}` as skeletons.
- Copy `scripts/preview-publish/discover-packages.ts` verbatim into `src/lib/packages.ts`. Update imports.
- Create empty `src/ci/bin.ts` with commander root and no subcommands.
- Add `@rivet/publish` to root `pnpm-workspace.yaml`.
- Run `pnpm install`.
- Verify `pnpm --filter=@rivet/publish ci --help` works.

**Not touched yet**: existing preview-publish and release scripts still in place, both workflows still use them.

### Step 2: Port `publish-all.ts` → `src/lib/npm.ts`

- Move `publish-all.ts` logic into `src/lib/npm.ts`. Consume `discoverPackages` from `src/lib/packages.ts`.
- Add `ci publish-npm` subcommand to `src/ci/bin.ts`.
- Change `.github/workflows/preview-publish.yaml` to call `pnpm --filter=@rivet/publish ci publish-npm` instead of `pnpm --filter=preview-publish exec tsx publish-all.ts`.
- Verify on a test PR.
- Delete `scripts/preview-publish/publish-all.ts`.

### Step 3: Port `bump-versions.ts` → `src/lib/version.ts#bumpPackageJsons`

- Move logic. Wire `ci bump-versions` subcommand.
- Swap preview workflow call.
- Delete `scripts/preview-publish/bump-versions.ts`.

### Step 4: Port R2 utils → `src/lib/r2.ts`

- Copy R2 client factory from `scripts/release/utils.ts`. Drop `listReleasesObjects`/`deleteReleasesPath` unless a caller needs them.
- Port `copyReleasesPath` (the R2-tagging-header workaround).
- Add `uploadArtifactsToR2`, `copyR2Path`, `uploadInstallScripts`.
- Add `ci upload-r2` and `ci copy-r2` subcommands.

### Step 5: Port Docker helpers → `src/lib/docker.ts`

- Copy `createManifestFromArch` + `retagManifest` from current `scripts/release/docker.ts`. Drop the `useVersionManifest` reuse branch. Inline the `{slim, full}` loop.
- Add `ci docker-manifest` and `ci docker-retag` subcommands.

### Step 6: Port git helpers → `src/lib/git.ts`

- Copy `validateClean`, `createAndPushTag`, `createGitHubRelease` from `scripts/release/git.ts`.
- Add `ci git-tag` and `ci gh-release` subcommands.

### Step 7: Port `update_version.ts` → `src/lib/version.ts#updateSourceFiles`

- Move source-file rewrite logic.
- Refactor `package.json` globs to use `discoverPackages()` instead of raw globbing.
- Keep `Cargo.toml` + examples + protocol version updates as file-level regex rewrites.

### Step 8: Write `src/local/cut-release.ts`

- Port the linear `main.ts` flow (minus phases, minus CI steps).
- Add `release` script to `package.json`.
- Verify `pnpm --filter=@rivet/publish release --version X.Y.Z --dry-run` locally (add a `--dry-run` that skips git push + workflow trigger).

### Step 9: Add remaining CI subcommands

- `ci context-output`, `ci comment-pr`.

### Step 10: Write merged `.github/workflows/publish.yaml`

- Copy current `preview-publish.yaml` as the base.
- Add `workflow_dispatch` trigger with `version` + `latest` inputs.
- Add `context` job.
- Gate `build_mode`, `build_frontend`, `cargo_build_mode` on `needs.context.outputs.trigger`.
- Add `docker-images` job (currently release-only).
- Wire `publish` job to call the new `ci` subcommands.
- Add release-only tail steps with `if: trigger == 'release'`.
- Add preview-only `comment-pr` step with `if: trigger == 'pr'`.
- Keep both `release.yaml` and `preview-publish.yaml` in place for now.

### Step 11: Cutover

- Trigger `publish.yaml` on a test PR. Verify all preview behaviors still work.
- Trigger `publish.yaml` manually via `workflow_dispatch` with a test pre-release version (e.g. `2.5.0-test.1`). Verify release tail runs correctly.
- Once green, delete `.github/workflows/release.yaml` and `.github/workflows/preview-publish.yaml`.
- Update `scripts/release/main.ts` `triggerWorkflow` call to target `publish.yaml`. (Though at this point the old `main.ts` is already replaced by `cut-release.ts`.)

### Step 12: Delete old scripts

- `rm -rf scripts/release/ scripts/preview-publish/`.
- Remove stale pnpm-workspace entries.
- Update any docs that reference old script paths.

## Risks and mitigations

### R1: First release under the new flow fails after partial npm publish

**Risk**: `publish-npm` succeeds for N of M packages, then `copy-r2` fails. The registry is now in a half-published state. Retrying the release-tail steps is fine, but we need idempotency.

**Mitigation**: `publishAll()` already treats "already published" as success (not failure). Re-running the workflow at the same sha republishes cleanly. The release-tail steps (`copy-r2`, `docker-retag`, `git-tag`, `gh-release`) are all idempotent (force-push for git tag, `gh release edit` for existing releases). The one non-idempotent concern is `git-tag` pushing a tag someone else already moved, but we already force-push today.

### R2: Context computation drift between local cut and CI

**Risk**: `cut-release.ts` computes `latest` locally and passes it as a workflow input. CI re-resolves context and computes a different value.

**Mitigation**: The CI-side `resolveContext()` reads the `latest` input directly; it never re-computes. All release-mode context fields come from workflow inputs, not from CI re-computation. Only preview-mode context fields are synthesized.

### R3: R2 upload from CI worker vs from script needs different auth

**Risk**: Current `release.yaml` uses inline `aws s3 cp` with env vars. The new flow routes through `src/lib/r2.ts` which has a 1Password fallback. If someone accidentally invokes that fallback in CI (no 1P CLI installed), it fails confusingly.

**Mitigation**: `r2.ts` must check env vars first and only fall back to 1Password when running interactively (e.g. detect `process.env.CI` and skip the fallback entirely). Tests: run `ci upload-r2` with env vars unset in a non-CI shell → should fall back; run with `CI=true` env vars unset → should fail with clear error message.

### R4: `docker-manifest` creates multi-arch manifests before both per-arch pushes land

**Risk**: `publish` job depends on `docker-images` job. `docker-images` has a matrix of two arches. If the matrix strategy uses `fail-fast: false` and one arch fails, the publish step still runs and `docker buildx imagetools create` fails because only one arch exists.

**Mitigation**: Use `needs.docker-images.result == 'success'` gate on publish (stronger than the current `!= 'failure'`). Alternatively, keep the weaker gate but have `docker-manifest` fail cleanly with "one or more per-arch images missing" when imagetools errors.

### R5: PR from fork can't access secrets

**Risk**: `pull_request` from a fork has no access to `NPM_TOKEN`, `DOCKER_CI_*`, `R2_*`, or `DEPOT_TOKEN`. Current preview-publish.yaml has this same issue and we've just accepted that forks don't get previews.

**Mitigation**: Keep the current behavior — skip publish job entirely for forks. Add `if: github.event.pull_request.head.repo.fork != true` on all publish-side steps that need secrets. Document this in the workflow header comment.

### R6: `workflow_dispatch` from a non-main branch

**Risk**: `cut-release.ts` calls `gh workflow run publish.yaml --ref <branch>`. CI runs against that branch, which may have stale `version` in package.jsons relative to `update_version.ts`. Tags get pushed from the branch HEAD.

**Mitigation**: Keep the current behavior — release is cut from a branch, the branch is pushed (via `gt submit` or `git push`), then CI runs against it. `cut-release.ts` commits the version bump on the current branch before triggering the workflow. The workflow checks out that ref. Everything stays consistent. Document that release cuts happen from main or a release branch.

### R7: `pnpm install --frozen-lockfile` fails after `bumpPackageJsons`

**Risk**: `bumpPackageJsons` rewrites every `package.json`. If CI runs `pnpm install --frozen-lockfile` *after* the bump, lockfile drift kills the install.

**Mitigation**: CI runs `pnpm install --frozen-lockfile` *before* bump. The bump only touches `version` fields on publishable packages — it does not change dependency specs (except `workspace:*` → concrete version for published packages, which pnpm already handles via the publish flow, not install flow). Verify: after `bumpPackageJsons`, do NOT run `pnpm install` again.

### R8: Discovery misses new package locations

**Risk**: `discoverPackages()` walks specific paths (`rivetkit-typescript/packages/rivetkit-native/npm`, `rivetkit-typescript/packages/engine-cli/npm`, `engine/sdks/typescript`, `shared/typescript`, pnpm workspace). A new package added outside these paths silently gets missed.

**Mitigation**: Add a smoke test in `packages.ts` that asserts the expected packages are present (e.g. `rivetkit`, `@rivetkit/react`, `@rivetkit/engine-cli`). Fail `ci bump-versions` with a clear error if any expected root package is missing.

### R9: `optionalDependencies` injection on `@rivetkit/engine-cli` adds a platform it doesn't support

**Risk**: `META_PACKAGES` says inject `@rivetkit/engine-cli-*` platform packages. Today engine-cli has four platforms. rivetkit-native has six. If we accidentally injection a platform that doesn't have a publishable package, the meta package becomes uninstallable.

**Mitigation**: The injection logic uses `packages.filter(p => p.name.startsWith(platformPrefix))` — it only injects platforms that are actually discovered. Add a sanity check: meta packages must have at least one platform package, else fail bump-versions.

### R10: Force-push git tag clobbers a real release tag

**Risk**: `git tag -f v${version}` + force push. Someone running the release cut with a stale checkout could overwrite a newer release tag.

**Mitigation**: `cut-release.ts` validates that the current branch HEAD is ahead of or equal to the latest `vX.Y.Z` tag before triggering. If not, refuse. This is a new check not in today's script. Low cost, high safety.

### R11: Engine debug builds get pushed to Docker Hub as `rivetdev/engine:slim-{sha}`

**Risk**: Preview PRs push debug-mode engine images to Docker Hub with the same tag shape as release mode images. A user pulling `rivetdev/engine:slim-abc1234` can't tell whether it's debug or release.

**Mitigation**: Option A — tag preview images with a different prefix (`rivetdev/engine:slim-preview-{sha}`). Option B — accept that preview images exist alongside release images at the same tag prefix and document it. Option C — push preview images to ghcr.io instead of Docker Hub, leaving Docker Hub release-only.

**Decision needed**: discuss with the user. Default to Option C (ghcr.io for preview, Docker Hub for release) because it avoids polluting the Docker Hub namespace with ephemeral tags.

### R12: Docker Hub rate limits / tag retention

**Risk**: Pushing `{slim,full}-{sha}-{arch}` + multi-arch manifest on every PR creates a huge number of tags. Docker Hub has tag retention costs and pull rate limits. Current preview-publish doesn't push Docker images so this is new load.

**Mitigation**: Tied to R11. Push preview to ghcr.io (free for public repos), Docker Hub for release only. Zero new load on Docker Hub.

### R13: Release-only steps run on a rerun of a successful workflow

**Risk**: Someone reruns a successful release workflow. `git-tag` force-pushes the same tag (fine). `gh-release` already handles existing releases via `gh release edit`. `copy-r2` overwrites (fine). `docker-retag` re-creates manifests (fine). `publish-npm` already-published → no-op (fine). This is actually fine.

**Mitigation**: None needed. Document that reruns are safe.

### R14: Version ownership drift

**Risk**: `updateSourceFiles` updates `Cargo.toml` workspace version, but misses a `version = ` in a sub-crate or a `package.json` in a location discovery doesn't know about.

**Mitigation**: The spec enforces that `bumpPackageJsons` uses `discoverPackages()` as the source of truth for which `package.json`s to touch, and `updateSourceFiles` handles everything else. Keep the current `update_version.ts` regex rules for `Cargo.toml`-level files (they're stable). Add a post-commit hook in `cut-release.ts` that greps the repo for `version = "<old>"` patterns and warns if any remain.

### R15: `publish-npm` parallelism hits npm registry rate limits

**Risk**: `--parallel 16` means 16 simultaneous `npm publish` calls. npm has undocumented rate limits.

**Mitigation**: Current preview-publish uses `--parallel 16` without issue. Keep the setting but make it tunable via workflow env var. If rate limiting becomes a problem, drop to 8 or 4.

### R16: Lost invariant — "`src/local` is never called by CI"

**Risk**: Someone adds a new local flow under `src/local/` and then accidentally wires it into a CI step.

**Mitigation**: ESLint `no-restricted-imports` rule (per the spec) plus a dedicated CI check: `grep -rn "src/local" .github/workflows/` should return zero matches.

## Open questions for the user

1. **Preview Docker images: Docker Hub or ghcr.io?** (R11/R12) — recommend ghcr.io for preview, Docker Hub for release.
2. **Keep R2 install scripts?** If `@rivetkit/engine-cli` is the canonical engine distribution via npm, is `curl install.sh | sh` still needed? Or does `install.sh` just run `npm install -g @rivetkit/engine-cli` under the hood?
3. **Delete `@rivetkit/devtools` R2 upload?** Confirm nothing consumes `rivet/{version}/devtools/` outside of "it got uploaded once." If nothing uses it, delete `build-artifacts.ts` entirely (current spec does this).
4. **Release tail parallelism.** Current spec runs release tail steps (`copy-r2`, `docker-retag`, `git-tag`, `gh-release`) sequentially in one job. They're independent and could run as four parallel jobs. Worth it? Probably not — they're fast.
5. **Dry-run mode.** Should `ci bump-versions` and `ci publish-npm` have a `--dry-run` flag for local testing? Adds complexity. Probably yes for `bump-versions` (already exists in `bump-versions.ts`), no for `publish-npm` (npm has `--dry-run` natively).

## Success criteria

1. `publish.yaml` on a test PR publishes preview packages to npm, pushes debug engine images to ghcr.io, uploads engine binaries to R2 at `rivet/{sha}/engine/`, and posts a PR comment. Time-to-green ≤ current preview-publish.
2. `publish.yaml` triggered via `workflow_dispatch` with `version=2.5.0-test.1` publishes packages to npm with tag `rc`, pushes release engine images, uploads R2 at `rivet/{sha}/engine/`, copies R2 to `rivet/2.5.0-test.1/engine/`, retags Docker to `slim-2.5.0-test.1`, creates git tag `v2.5.0-test.1`, creates GitHub release marked prerelease. All steps succeed.
3. `scripts/release/` and `scripts/preview-publish/` directories are deleted.
4. `scripts/publish/` passes ESLint with the boundary rules enforced.
5. `cut-release.ts --version X.Y.Z --dry-run` runs locally without touching git or triggering the workflow.
6. Total CI wall clock for a preview PR is within 10% of current preview-publish.
7. No flags removed during cleanup are referenced anywhere in the repo (grep for `--phase`, `--only-steps`, `--reuse-engine-version`).
