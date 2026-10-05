/** @type {import('@ladle/react').UserConfig} */
export default {
	viteConfig: ".ladle/vite.config.ts",
	addons: {
		// The dashboard ships dark-only (`<html class="dark">`), so match that by
		// default; the theme toggle switches to the light tokens.
		theme: {
			enabled: true,
			defaultState: "dark",
		},
	},
};
