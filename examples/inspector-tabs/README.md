# Inspector Tabs

A working example of custom inspector tabs shipped alongside a Rivet Actor. A counter actor declares two author-defined tabs (`Counter`, `Info`) and hides the built-in `Queue` tab from the dashboard inspector strip.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/inspector-tabs
npm install
npm run dev
```

Open the [Rivet dashboard](https://dashboard.rivet.dev), navigate to your counter actor, and the **Counter** and **Info** tabs appear alongside the built-ins. The built-in **Queue** tab is hidden because this actor doesn't use queues.

## Features

- **Custom tabs ship with the actor.** Declared in `inspector.tabs[]` on the actor definition. The dashboard discovers them via `GET /inspector/tab-config` and renders them in the inspector strip.
- **No bundler.** Each tab is a single static `index.html` with inline `<script>`. rivetkit serves the bytes as-is — but the `source` field accepts any directory, so Vite/webpack/React/Vue build outputs work the same way.
- **Hide built-in tabs.** `{ id: "queue", hidden: true }` removes the queue tab from the strip when the actor doesn't use that subsystem.
- **Custom icons.** `icon: "tag"` on a tab descriptor maps to a dashboard glyph; unknown ids fall back to a neutral icon.
- **Read state, invoke actions.** Tabs hit `/inspector/state`, `/inspector/action/<name>`, `/inspector/rpcs`, and the rest of the inspector HTTP API with `Authorization: Bearer ${authToken}` — the same token the built-in inspector tabs use.
- **Dashboard-native styling.** Both tabs link `../../tab.css`, a stylesheet the engine serves that mirrors the dashboard's design tokens under `--rivet-*` CSS variables. Tabs look at home in the inspector without any custom CSS.
- **Live light/dark theme.** The dashboard signals its active theme in the `v1Init` postMessage; tabs apply the `dark` class to `<html>` and flip in lockstep when the user toggles theme in the dashboard's user dropdown.
- **TypeScript types.** Tabs built with TypeScript can `import type { V1Init, InspectorStateResponse, ... } from "rivetkit/inspector-tab"` for compile-time safety on the handshake and HTTP shapes.

## Implementation

- **Actor + tab declarations** ([`src/index.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/inspector-tabs/src/index.ts)): a counter actor whose `inspector.tabs[]` array declares both custom tabs plus the queue-hide modifier. Inline JSDoc walks through every property (`id`, `label`, `source`, `icon`, `hidden`) and notes that `source` can point at any built-asset directory.
- **Counter tab** ([`inspector-tabs/counter/index.html`](https://github.com/rivet-dev/rivet/tree/main/examples/inspector-tabs/inspector-tabs/counter/index.html)): live count, +1 / +5 / +10 / reset buttons, recent-changes history. Demonstrates invoking actor actions from a tab and polling state on a 1 s tick. Heavy inline comments walk through the postMessage contract and auth flow — this file is the deeper tutorial.
- **Info tab** ([`inspector-tabs/info/index.html`](https://github.com/rivet-dev/rivet/tree/main/examples/inspector-tabs/inspector-tabs/info/index.html)): actor id, registered actions, raw state JSON. Demonstrates calling multiple inspector endpoints in parallel with `Promise.all`. Leaner comments; pairs with Counter for the basics.

### The handshake contract

Every custom tab is a normal web page. The contract between the tab and the dashboard has five pieces:

1. **Read `?shellOrigin` from the URL.** The dashboard sets it to its own origin. Use it as the target origin on every outbound `postMessage` AND validate inbound `event.origin` against it — without this check any third-party page that frames the tab can forge an `init` message.
2. **Listen for `{ type: "init", v: 1, actorId, authToken, theme? }`** on `window.addEventListener("message", ...)`. Drop messages whose `event.origin` doesn't match the trusted shell origin. Accept late `init` messages — the dashboard re-issues `init` whenever the token or theme changes.
3. **Mirror the dashboard theme.** `document.documentElement.classList.toggle("dark", (msg.theme ?? "dark") === "dark")`. The shared stylesheet's CSS variables drive off that class.
4. **Send `{ type: "ready", v: 1 }` to `window.parent`** once the tab mounts. If the dashboard doesn't see `ready` within 8 s it shows "Inspector UI didn't load."
5. **Fetch inspector endpoints with `Authorization: Bearer ${authToken}`** — and post `{ type: "token-refresh-needed", v: 1 }` on a 401. Don't silently retry; wait for the next `init` instead.

Inspector data routes live two directories up from the bundle: the tab loads at `/inspector/custom-tabs/<id>/`, so `../../state` resolves to `/inspector/state` on the same actor. The shared stylesheet is at `../../tab.css`. Absolute paths like `/inspector/state` would resolve to the engine root and 404 — always use the relative form.

### TypeScript

If you're building the tab with TypeScript (Vite, Webpack, etc.), pull the shapes from rivetkit:

```ts
import type {
  V1Init,
  V1Ready,
  V1TokenRefreshNeeded,
  ShellToTabMessage,
  TabToShellMessage,
  InspectorStateResponse,
  InspectorActionResponse,
  InspectorRpcsResponse,
} from "rivetkit/inspector-tab";
```

The module is types-only — no runtime cost, no import in the emitted bundle.

### Shared stylesheet

`../../tab.css` exposes the dashboard's design tokens under a `--rivet-` prefix. Color tokens come in two forms:

```css
.my-card {
  background: var(--rivet-card);                       /* pre-wrapped */
  border:     1px solid var(--rivet-border);
  color:      var(--rivet-foreground);
  padding:    var(--rivet-space-4);
  border-radius: var(--rivet-radius-md);
}

.my-overlay {
  background: hsl(var(--rivet-background-raw) / 0.6);  /* raw HSL, alpha-aware */
}
```

It also ships sensible defaults for `<body>`, `<button>`, `<input>`, and helper classes (`.rivet-card`, `.rivet-section-header`, `.rivet-section-body`, `.rivet-muted`, `.rivet-mono`, `.rivet-error`, `button.rivet-primary`, `button.rivet-danger`). A bare tab using `../../tab.css` looks at home in the inspector without any custom CSS.

You can ignore the stylesheet entirely and bring your own — the inspector tab API does not require it.

## Inspector endpoints custom tabs can call

The actor exposes these unauthenticated bundle paths and authenticated data paths:

| Method | Path | Notes |
|---|---|---|
| `GET` | `/inspector/tab-config` | Tab descriptor list. Public. |
| `GET` | `/inspector/custom-tabs/<id>/*` | Static assets from the tab's `source` directory. Public. |
| `GET` | `/inspector/tab.css` | Shared dashboard-token stylesheet. Public. |
| `GET` | `/inspector/state` | Current actor state. Auth required. |
| `PATCH` | `/inspector/state` | Replace actor state. Auth required. |
| `POST` | `/inspector/action/<name>` | Invoke an action with `{ args: [] }` or `{ properties: {} }`. Auth required. |
| `GET` | `/inspector/rpcs` | Names of registered actions. Auth required. |
| `GET` | `/inspector/connections` | Active client connections. Auth required. |
| `GET` | `/inspector/queue` | Queue snapshot. Auth required. |

## Resources

Read more about [custom inspector tabs](/docs/actors/inspector-tabs), [the inspector HTTP API](/docs/actors/debugging), [actions](/docs/actors/actions), and [state](/docs/actors/state).

## License

MIT
