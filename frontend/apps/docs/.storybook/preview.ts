import "./styles.css";

import { withThemeByClassName } from "@storybook/addon-themes";
import type { Preview, ReactRenderer } from "@storybook/react";

const preview: Preview = {
	parameters: {
		controls: {
			matchers: {
				color: /(background|color)$/i,
				date: /Date$/i,
			},
		},
	},
	decorators: [
		withThemeByClassName<ReactRenderer>({
			themes: {
				light: "",
				dark: "dark",
			},
			defaultTheme: "dark",
		}),
	],
};

export default preview;
