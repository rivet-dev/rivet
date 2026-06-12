import { actor, event, setup } from "rivetkit";

// ============================================================================
// Counter actor with author-declared inspector tabs.
//
// This example actor is small on purpose: a counter with two actions and a
// modest history. The interesting part is the `inspector` config at the
// bottom of the actor definition — it declares two custom inspector tabs
// that ship with this actor and tells the dashboard to hide the built-in
// "Queue" tab (since the counter doesn't use a queue).
// ============================================================================

export type Tick = { value: number; at: number };

export const counter = actor({
	// Persistent actor state. The "Counter" inspector tab reads this via
	// `GET /inspector/state` and updates it via the `increment` / `reset`
	// actions below.
	state: {
		value: 0,
		history: [] as Tick[],
	},

	events: {
		valueChanged: event<number>(),
	},

	actions: {
		// Invoked from the custom tab via
		// `POST /inspector/action/increment` with body `{ args: [amount] }`.
		increment: (c, amount: number) => {
			c.state.value += amount;
			c.state.history.push({ value: c.state.value, at: Date.now() });
			if (c.state.history.length > 50) {
				c.state.history.shift();
			}
			c.broadcast("valueChanged", c.state.value);
			return c.state.value;
		},

		// Invoked from the custom tab via `POST /inspector/action/reset`.
		reset: (c) => {
			c.state.value = 0;
			c.state.history = [];
			c.broadcast("valueChanged", 0);
			return 0;
		},
	},

	// ========================================================================
	// `inspector.tabs[]` — author-declared inspector tabs.
	// ========================================================================
	//
	// Each entry is one of two kinds:
	//
	//   • Custom tab: `{ id, label, source, icon? }` — adds a new tab whose
	//     UI is the directory of static files at `source`.
	//   • Hide modifier: `{ id, hidden: true }` — removes a built-in tab
	//     from the dashboard strip. `id` must be one of the six built-in
	//     ids: workflow | database | state | queue | connections | console.
	//
	// Validation runs at registry construction. Misconfiguration (missing
	// directory, duplicate id, custom id colliding with a built-in, slash
	// in an id, etc.) throws before the actor starts so problems surface
	// loudly instead of silently corrupting the inspector strip.
	//
	// ========================================================================
	// Property reference
	// ========================================================================
	//
	// id (required)
	//   Stable identifier for this tab. Used in two places:
	//     1. The bundle URL: `/inspector/custom-tabs/<id>/`.
	//     2. The dashboard tab-strip key.
	//   Custom ids must match `/^[A-Za-z0-9_-]+$/` (slashes, dots, and
	//   spaces are rejected because the URL splits on `/`). Two tabs on the
	//   same actor cannot share an id. A custom id cannot equal a built-in
	//   id — use `{ id: "queue", hidden: true }` to hide a built-in instead.
	//
	// label (required for custom)
	//   The human-readable name shown in the tab strip. Free-form string.
	//
	// source (required for custom)
	//   Path to a directory of static assets the dashboard serves at
	//   `/inspector/custom-tabs/<id>/*`. Resolved relative to the actor
	//   process's cwd. rivetkit reads files from this directory on
	//   demand — there is no copy step, no bundling step, no service
	//   worker. The bytes you put in this directory are the bytes the
	//   browser sees.
	//
	//   The directory layout matches a normal web page: `index.html` is
	//   served at `/`, any other file is served at its relative path
	//   (`/style.css`, `/assets/logo.svg`, `/chunks/main-abcd.js`, etc.).
	//
	//   You can absolutely use a bundler — point `source` at the build
	//   output. Some shapes that work:
	//
	//     ./inspector-tabs/leaderboard          (raw HTML + inline JS,
	//                                            like this example)
	//     ./inspector-tabs/leaderboard/dist     (Vite / Webpack / Rspack
	//                                            output directory after
	//                                            `vite build` / `webpack`)
	//     ../my-tab-app/dist                    (separate package built
	//                                            into the monorepo)
	//
	//   React, Vue, Svelte, Solid, htmx, vanilla — none of it matters to
	//   rivetkit. The framework runs entirely in the iframe; rivetkit
	//   only sees the static bytes.
	//
	//   Security note: rivetkit canonicalizes both the requested path and
	//   the source directory, then rejects requests whose canonical form
	//   escapes the source root. Symlink-mediated path traversal returns
	//   400 instead of leaking files outside the bundle.
	//
	//   Bundle bytes are served WITHOUT authentication (the dashboard
	//   reads tab descriptors before any token check could succeed).
	//   Don't inline secrets, API keys, or sensitive source-map paths
	//   into your tab bundle — treat the bundle as a public website
	//   asset. Authenticated calls inside the tab still use the
	//   `Authorization: Bearer` header from the postMessage handshake.
	//
	// icon (optional, custom only)
	//   String identifier the dashboard maps to a glyph. The current
	//   registry resolves these:
	//     workflow | database | state | queue | plug | terminal | tag | logs
	//   Any other string falls back to a neutral question-mark icon, which
	//   is also what you get if you omit `icon` entirely.
	//
	// hidden (required for hide modifier)
	//   Must be exactly `true`. Marks the entry as "hide built-in"; the
	//   dashboard removes the matching built-in tab from the strip.
	//   `id` MUST be one of the six built-in ids when `hidden: true`.
	//   Hiding is additive on top of capability gating — the built-in
	//   tab disappears regardless of whether the actor would have
	//   exposed it.
	//
	// ========================================================================
	inspector: {
		tabs: [
			{
				id: "counter",
				label: "Counter",
				icon: "tag",
				source: "./inspector-tabs/counter",
			},
			{
				id: "info",
				label: "Info",
				// `icon` omitted on purpose: shows the neutral fallback
				// glyph next to the "Info" label in the dashboard strip.
				source: "./inspector-tabs/info",
			},
			{
				// Hide the built-in "Queue" tab. The counter actor doesn't
				// use the queue subsystem, so the tab would be empty.
				id: "queue",
				hidden: true,
			},
		],
	},
});

// Register actors for use: https://rivet.dev/docs/setup
export const registry = setup({
	use: { counter },
});

registry.start();
