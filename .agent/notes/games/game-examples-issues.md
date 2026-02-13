# Game Examples: Issues Encountered

Last updated: 2026-02-13

This is a running log of issues we hit while iterating on the `examples/game-*` projects, plus any context needed to reproduce and fix them.

## Runtime Issues

- `examples/game-fps-arena`: blank/black screen when joining match.
  - Symptom: UI renders, tick increments, but viewport stays blank and only shows "click to lock".
  - Fix applied: ensure the app root fills the viewport (`#root { height: 100% }`) and fix menu/game layering so the WebGL canvas is visible.

- `examples/game-fps-arena`: input axes mismatch.
  - Symptom: forward/backwards inverted, yaw (left/right look) inverted, pitch OK.
  - Fix applied: correct forward/right vectors and add a client/server yaw convention conversion.

- `examples/game-fps-arena`: jerky camera rotation and movement.
  - Symptom: client looks jittery as if server corrections are fighting the client.
  - Likely causes to revisit:
    - too-aggressive server authority without client prediction/smoothing
    - sending inputs at a low rate without interpolation
    - applying server snapshots directly to the local player without reconciliation

- `examples/game-fps-arena`: holding space causes repeated hopping.
  - Symptom: player auto-hops repeatedly when space is held.
  - Fix applied: edge-trigger jump on keydown while grounded, and send `jump` to the server only once per press.

- `examples/game-fps-arena`: bullets/tracers initially too small and POV-originated.
  - Symptom: tracer is hard to see; visually originates from camera center rather than the gun.
  - Fix applied: render tracer with a blocky head + long semi-transparent streak, and spawn the visual tracer from a gun muzzle offset from the camera.
  - Note: server hit validation remains eye-based hitscan, while visuals originate at the muzzle for game feel.

- `examples/game-fps-arena`: other players' jumps not visible.
  - Symptom: local jump could be client-only; remote players never appear airborne.
  - Fix applied: add server-side jump velocity/gravity state and replicate.

- `examples/game-fps-arena`: obstacle collisions not working.
  - Symptom: player passes through boxes or collision differs between client and server.
  - Fix applied: shared obstacle layout and shared circle-vs-AABB resolution logic used by both client and server.

- `examples/game-fps-arena`: map visuals were box/texture-based and did not match the new grid-based collision world.
  - Symptom: world was still rendered as colored cubes with prototype textures, while server/client physics had moved to a tile/grid map.
  - Fix applied: render the arena using Kenney `prototype-kit` GLB tiles (floors, walls, thick platform tiles, narrow stairs, doorway indicators) driven by `WORLD_GRID`/`WORLD_RAMPS`.
  - Notes:
    - Prototype textures are no longer used for this example.
    - Keep a small negative-Y base plane under the tiles to avoid gaps and z-fighting.

- `examples/game-fps-arena`: deterministic tests got flaky when the map introduced a raised mid platform.
  - Symptom: vitest hitscan test could fail or time out depending on spawn/obstacles (straight-line move could get blocked by the mid platform).
  - Fix applied: prefer a set of known-good lane spawn points in `buildSpawn`, keeping the first two spawns on the same flat lane for deterministic hitscan.

- `examples/game-fps-arena`: single-shot only firing felt wrong for an arena shooter.
  - Symptom: holding mouse button did not keep firing.
  - Fix applied: client treats the weapon as automatic and repeatedly calls `fire` while LMB is held. The server stays authoritative and rate limits.

- `examples/game-fps-arena`: remote players appeared half buried in the floor and blaster models were not visible.
  - Symptom: other players looked like they were inside the ground; first-person gun stayed as the fallback box/cylinder or was invisible.
  - Fix applied:
    - offset the capsule fallback avatar so feet sit on y=0 (CapsuleGeometry is centered by default).
    - add the camera to the scene graph so the gun (attached to the camera) renders.
    - prefer a larger blaster model from Kenney `blaster-kit` (`blaster-r.glb`) for the first-person gun.
  - Follow-up fix applied:
    - replicate per-player avatar selection via `PlayerPublicState.avatar` and load the correct Kenney blocky character model per remote player.
    - attach a third-person blaster model to remote avatars (best-effort mount) so other players visibly hold a weapon.
    - add a lightweight walk cycle (limb swing when moving) and head pitch look for models with separate limb nodes.

- `examples/game-fps-arena`: prototype-kit tiles overlapped and clipped into each other.
  - Symptom: some floor pieces or indicators appeared to clip into the floor, especially around ramps/platforms.
  - Fix applied: do not place the thin floor tile under platforms or ramps, and lift doorway indicators slightly above the floor.

- `examples/game-fps-arena`: remote yaw interpolation flips and player facing mismatch.
  - Symptom: remote players flip when crossing a critical angle, and some models appear to face backwards.
  - Fix applied: rotate Kenney blocky character instances by `PI` to match the example's `-Z` forward convention and interpolate yaw with wrapped angle math (not linear lerp).

- `examples/game-fps-arena`: stepping off stairs onto adjacent platforms felt blocked.
  - Symptom: players can go partially up stairs but cannot smoothly walk onto the raised platform next to the stairs.
  - Fix applied: allow a small step-up onto platform surfaces during `resolveObstaclesXZ` when within `STEP_HEIGHT`.

- `examples/game-fps-arena`: world sometimes appeared empty due to GLB URLs with spaces.
  - Symptom: tile world and/or gun model fails to load, leaving a mostly empty scene.
  - Fix applied:
    - rename Kenney asset folders at download time to remove spaces (e.g. `Models/GLB format` -> `Models/glb`) to avoid static server path decoding issues.
    - update runtime paths to use the normalized folder names.
    - keep `encodeURI` in `GLTFLoader` URLs as an extra guard.
  - Verified: `/tmp/fps-prefab-world.png`

- `examples/game-fps-arena`: geometry loaded lazily, causing jarring transitions and silent placeholder fallbacks.
  - Symptom: menu could show before assets were ready; if GLBs failed to load, the world/gun could silently fall back or appear empty.
  - Fix applied: preload required Kenney geometry (prototype-kit tiles + blaster + blocky character) on initial page load, show a blocking loading screen before the menu, and show a fatal error screen if any geometry fails to load.
  - Verified: `/tmp/fps-loading-screen.png`, `/tmp/fps-after-loading-menu.png`

- `examples/game-idle-base`: crashes at runtime on this machine with Node `v24.13.0`.
  - Symptom: actor startup fails with `RuntimeError: memory access out of bounds` originating from `wa-sqlite` (`wa-sqlite-async.mjs`).
  - Status: still repros as of 2026-02-13. UI loads briefly, then server-side runtime crashes.
  - Suspect: Node 24 + WASM sqlite runtime incompatibility/regression.

- `examples/game-mmo-chunks`: runtime crash on this machine with Node `v24.13.0`.
  - Symptom: `Error: Failed to save actor state: BareError: (byte:0) too large buffer`.
  - Status: repro’d during `pnpm dev` session.
  - Suspect: actor state serialization exceeding limits, or a bug in the driver/storage layer.

- `examples/game-fps-arena`: dev server crash due to trace storage state growth.
  - Symptom: Vite dev session exits with `Failed to save actor state: BareError: (byte:0) too large buffer` originating from `ActorTracesDriver.set`.
  - Fix applied: use `createMemoryDriver()` in `src/actors/index.ts` so long dev sessions do not persist large trace KV blobs to disk.

- `examples/game-fps-arena`: add combat feedback and scoring semantics.
  - Change applied:
    - award `+10` points per kill (server authoritative) and broadcast a `kill` event to drive UI feedback.
    - slow visual tracers so bullets read at short range even though shots are hitscan.
    - add bullet-hole decals on world impacts when a shot does not hit a player.
    - add hitmarker (`X`) on hit, hurt vignette when the local player is hit, and killfeed/toast messages.
    - add Kenney audio (`sci-fi-sounds`) for shooting and getting hit, and use UI pack click sounds for UI interactions.
  - Files:
    - `examples/game-fps-arena/src/actors/match.ts`
    - `examples/game-fps-arena/src/types.ts`
    - `examples/game-fps-arena/frontend/App.tsx`
    - `examples/game-fps-arena/frontend/game/ThreeFpsView.tsx`
    - `examples/game-fps-arena/scripts/assets/download-assets.mjs`

- `examples/game-fps-arena`: page scroll + GC hitches during extended play.
  - Symptom: the page could scroll (unwanted for a fullscreen game) and the client could hitch after sustained firing.
  - Fix applied:
    - force `overflow: hidden` on `html, body, #root` to prevent scrollbars from appearing.
    - replace per-shot tracer mesh/material allocations with a bounded tracer pool and reuse bullet-hole decals with a ring buffer to avoid constant allocation/disposal churn.
    - stop deep-cloning resources for the "prop museum" and lane props by using static clones that share geometry/materials, reducing startup memory and GC pressure.
  - Files:
    - `examples/game-fps-arena/frontend/App.css`
    - `examples/game-fps-arena/frontend/game/ThreeFpsView.tsx`

- Various examples: RivetKit runtime warnings during dev.
  - Vite warning: dynamic imports in rivetkit dist cannot be analyzed (`vite:import-analysis`).
  - Log noise: `subscription does not exist in persist` warnings.
  - `baseline-browser-mapping` warns the mapping data is out of date.
  - These did not block the UI checks but they add noise and may hide real issues.

- `examples/game-fps-arena`: `unhandled actor start promise rejection` about missing actor name `town`.
  - Symptom: log says `no actor in registry for name town`.
  - Likely cause: stale persisted actor instance from another example or key collision across examples sharing a driver/data dir.

## Typecheck Issues

- `examples/game-idle-base`: TypeScript errors.
  - Symptom: `.then(setTop)` where the value was inferred as `Promise<LeaderboardEntry[]>`, plus `unknown` DB row typing from `c.db.execute`.
  - Fix applied: `await` the call and add safe row decoding/casting.

- `examples/game-io-arena`: TypeScript errors.
  - Symptom: `c.state` inferred as `unknown` inside actor actions; missing/insufficient typings for `d3-quadtree`; `.then(setRoomId)` mismatch.
  - Fix applied: add explicit `RoomState` return typing on `createState`, add minimal `d3-quadtree` module declaration, and `await`/narrow connections in frontend code.

## UI/UX Issues

- Menu alignment and background misalignment complaints (notably in `game-fps-arena`).
  - Fix applied: center menu panels; avoid overlapping layers; improve backdrop/scanline effect; ensure Kenney UI Pack texture is used as a subtle motif instead of a misaligned overlay.

- UI requirements applied across game examples:
  - Use Kenney UI Pack for buttons/controls.
  - Avoid monospaced fonts in UI chrome.
  - Avoid normal casing: UI chrome moved to `text-transform: uppercase`.

## Agent-Browser Verification Artifacts

Screenshots captured while verifying UI layouts:

- `examples/game-fps-arena`: `/tmp/game-fps-arena-ui.png`
- `examples/game-fps-arena` (post world/model update): `/tmp/fps-menu-updated.png`
- `examples/game-fps-arena` (in match, post world/model update): `/tmp/fps-in-match.png`
- `examples/game-fps-arena` (prototype-kit tiles): `/tmp/fps-tiles-menu.png`
- `examples/game-fps-arena` (in match, prototype-kit tiles): `/tmp/fps-tiles-in-match.png`
- `examples/game-fps-arena` (world fallback + encoded GLB URLs): `/tmp/fps-world-not-empty.png`
- `examples/game-idle-base`: `/tmp/game-idle-base-ui.png`
- `examples/game-io-arena`: `/tmp/game-io-arena-ui.png`
- `examples/game-mmo-chunks`: `/tmp/game-mmo-chunks-ui.png`
- `examples/game-npc-town-ai`: `/tmp/game-npc-town-ai-ui.png`
- `examples/game-party-cah`: `/tmp/game-party-cah-ui.png`
- `examples/game-tic-tac-toe`: `/tmp/game-tic-tac-toe-ui.png`
