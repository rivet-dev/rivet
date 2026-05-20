# Hub frontend Foundry restyle

Mirror the visual language of `sandbox-agent-v2/dhaka/frontend/packages/website` (Foundry) across `cairo/frontend`. Phased so each chunk can land independently.

## Status: code-complete pending visual QA.

## Foundry traits mirrored
- Dark mode only. Cool zinc-blacks (`#09090b`, `#0f0f11`, `#0c0c0e`) instead of cairo's warm-brown.
- IBM Plex Sans (body + headings) + IBM Plex Mono.
- Glass surfaces: `bg-white/[0.02] backdrop-blur-md border border-white/10`, hover border `white/20`.
- White-on-black primary button: not adopted — kept Rivet's orange primary (brand).
- `rounded-lg` (buttons / inputs) and `rounded-xl` (cards / modals).
- Orange accent at `#ff4f00`/`#ff5500` (Rivet brand).
- Utilities lifted: `.glass`, `.glass-strong`, `.glow-accent`, `.shine-top`, `.text-gradient-accent`.

## Non-goals
- Keep FontAwesome (`@rivet-gg/icons`); did not swap to lucide.
- Did not rewrite component file structure.
- Did not touch workflow-diagram (XYFlow) or Shiki code-highlight colors.

## Files touched

### Phase 1 — Foundation
- `frontend/packages/components/public/theme.css` — cool zinc HSL tokens, dropped light mode, IBM Plex `@import`.
- `frontend/src/components/theme.css` — same updates (less likely to be loaded but kept in sync).
- `frontend/src/components/tailwind-base.ts` — `fontFamily.sans` → IBM Plex Sans; added `fontFamily.mono`.
- `frontend/src/index.css` — added `.glass`, `.glass-strong`, `.glow-accent`, `.shine-top`, `.text-gradient-accent` in `@layer components`.

### Phase 2 — Primitives
- `frontend/src/components/ui/button.tsx` — base `rounded-lg`; rebuilt secondary/outline/ghost variants on `border-white/10` + hover `border-white/20` + `bg-white/[0.06]` hover bg.
- `frontend/src/components/ui/input.tsx` — `bg-white/[0.02]`, `border-white/10`, hover lift.
- `frontend/src/components/ui/textarea.tsx` — same.
- `frontend/src/components/ui/select.tsx` — same on trigger.
- `frontend/src/components/ui/card.tsx` — `rounded-xl border-white/10`.
- `frontend/src/components/ui/dialog.tsx` — overlay `bg-background/70 backdrop-blur-md`; content `rounded-xl border-white/10 bg-card shadow-2xl`.

### Phase 3 — Chrome
- `frontend/src/app/layout.tsx` — sidebar `border-r border-white/10`; `HeaderLink` active state `bg-white/[0.06]`.
- `frontend/src/app/actor-builds-list.tsx` — active state `bg-white/[0.06]`.
- `frontend/src/components/ui/popover.tsx` — glass surface, `rounded-lg`.
- `frontend/src/components/ui/dropdown-menu.tsx` — glass surface on both Content + SubContent, `rounded-lg`.
- `frontend/src/components/ui/tooltip.tsx` — glass surface.
- `frontend/src/components/ui/sonner.tsx` — `theme="dark"`, glass toast surface, `rounded-lg`.
- `frontend/src/components/ui/sheet.tsx` — overlay `bg-background/70 backdrop-blur-md`.

### Phase 4 — New flow surfaces
- `frontend/src/app/forms/create-project-form.tsx` — type cards: glass + `border-white/10` → `border-primary glow-accent` when selected.
- `frontend/src/app/actors-grid.tsx` — `GridCard` glass + hover border-shift; outer wrapper `rounded-xl border-white/10`; build icon container `bg-white/[0.06]`.
- `frontend/src/app/agent-panel.tsx` — chat bubbles: agent `bg-white/[0.04]`, user `bg-primary/15 border-primary/20`; avatar `bg-white/[0.06]`; bubbles `rounded-lg`.
- `frontend/src/app/namespace-agent-layout.tsx` — agent column wrapper `rounded-xl border-white/10`.

### Phase 5 — Polish
- `frontend/src/components/ui/typography.tsx` — H1 `text-2xl lg:text-3xl tracking-tight`; H2 `text-2xl`; H3 `text-xl` (was H1 `text-xl lg:text-4xl`, H2 `text-3xl`, H3 `text-2xl`).
- `frontend/src/components/mdx/index.tsx` — `Note` callout swapped from `text-gray-*` literals to tokens.
- `frontend/src/components/actors/guard-connectable-inspector.tsx` — `text-gray-600` → `text-muted-foreground`.

## Visual QA checklist (Task #19)
Walk these routes at `https://local.staging.rivet.dev`. Screenshot anything that regresses.

- `/login` — login form, Google button, Turnstile presence.
- `/orgs/$org` — context switcher, project list.
- `/orgs/$org/new` — new project creation form (the new flow I built).
- `/orgs/$org/projects/$project` — namespace list redirect.
- `/orgs/$org/projects/$project/ns/$namespace` — actors grid landing (the new flow).
- `/orgs/$org/projects/$project/ns/$namespace?n=…` — list+inspector view, tabs (Config, State, Database, Logs, Queue, Workflow, Connections).
- `/orgs/$org/projects/$project/ns/$namespace/settings` — settings page.
- `/orgs/$org/projects/$project/ns/$namespace/billing` — billing.
- Agent panel toggle (bottom-right pill) — collapse/expand, chat bubbles.
- Modals reachable via header dropdowns: Feedback, CreateNamespace, CreateOrganization, OrgMembers.
- Toasts (any mutation that triggers one).

## Approach
- One PR per phase (or all-in-one if user prefers).
- Verify in browser at `https://local.staging.rivet.dev` after each phase (vite + cloudflared tunnel running locally).
