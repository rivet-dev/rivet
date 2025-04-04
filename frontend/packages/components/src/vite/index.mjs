import * as shiki from "shiki";
import { transformerNotationFocus } from "@shikijs/transformers";

/**
 * 
 * @returns {import("vite").Plugin}
 */
export async function viteShikiTransformer(){
	const cssVariableTheme = shiki.createCssVariablesTheme({
		name: "css-variables",
		variablePrefix: "--shiki-",
		variableDefaults: {},
		fontStyle: true,
	});

	let highlighter
	return {
		name: "shiki",
		async transform(code, id) {
			if (id.includes("?shiki")) {
				highlighter ??= await shiki.getSingletonHighlighter({
					themes: [cssVariableTheme],
					langs: [
						"bash",
						"batch",
						"cpp",
						"csharp",
						"docker",
						"gdscript",
						"html",
						"ini",
						"js",
						"json",
						"powershell",
						"ts",
						"typescript",
						"yaml",
						"http",
						"prisma",
					],
				});

				const params = new URLSearchParams(id.split("?")[1]);
				const output = highlighter.codeToHtml(code, {
					lang: params.get("lang") ?? "bash",
					theme: "css-variables",
					transformers: [transformerNotationFocus()],
				});
				return `export default ${JSON.stringify(
					output,
				)};export const source = ${JSON.stringify(code)}`;
			}
		},
	};
}
