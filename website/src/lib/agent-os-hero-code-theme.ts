import baseTheme, { BACKGROUND_PROPERTIES } from "./textmate-code-theme";

const theme = structuredClone(baseTheme) as any;

theme.name = "agentos-hero-light";
theme.type = "light";

const surface = "#fafafa";
const foreground = "#52525b";

theme.colors = {
	...theme.colors,
	foreground,
	"editor.background": surface,
	"editor.foreground": foreground,
};

for (const key of BACKGROUND_PROPERTIES) {
	theme.colors[key] = surface;
}

theme.tokenColors = [
	...theme.tokenColors,
	{
		name: "agentOS hero comments",
		scope: ["comment", "punctuation.definition.comment"],
		settings: {
			foreground: "#a1a1aa",
			fontStyle: "italic",
		},
	},
	{
		name: "agentOS hero keywords",
		scope: [
			"keyword",
			"keyword.control",
			"keyword.control.flow",
			"keyword.operator.expression",
			"storage",
			"storage.type",
		],
		settings: {
			foreground: "#9333ea",
		},
	},
	{
		name: "agentOS hero strings",
		scope: [
			"string",
			"string.quoted",
			"string.template",
			"constant.other.symbol",
		],
		settings: {
			foreground: "#059669",
		},
	},
	{
		name: "agentOS hero functions",
		scope: [
			"entity.name.function",
			"support.function",
			"variable.function",
			"meta.function-call",
		],
		settings: {
			foreground: "#2563eb",
		},
	},
	{
		name: "agentOS hero numbers",
		scope: ["constant.numeric", "constant.language.boolean", "constant.language.null"],
		settings: {
			foreground: "#ea580c",
		},
	},
	{
		name: "agentOS hero variables",
		scope: [
			"variable",
			"variable.other.readwrite",
			"support.variable",
			"entity.name.type",
			"meta.object-literal.key",
		],
		settings: {
			foreground: "#0e7490",
		},
	},
	{
		name: "agentOS hero punctuation",
		scope: [
			"punctuation",
			"meta.brace",
			"meta.delimiter",
			"keyword.operator",
			"delimiter",
		],
		settings: {
			foreground: "#71717a",
		},
	},
];

export default theme;
