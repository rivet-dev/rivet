import { recmaPlugins } from "./recma.mjs";
import { rehypePlugins } from "./rehype.mjs";
import { remarkPlugins } from "./remark.mjs";

export const config = {
	extension: /\.mdx?$/,
	options: {
		remarkPlugins,
		rehypePlugins,
		recmaPlugins,
	},
};
