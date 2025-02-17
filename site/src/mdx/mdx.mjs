import { recmaPlugins } from "./recma.mjs";
import { rehypePlugins } from "./rehype.mjs";
import { remarkPlugins } from "./remark.mjs";

/**
 * @type {import('@next/mdx').NextMDXOptions}
 */
export const config = {
	extension: /\.mdx?$/,
	options: {
		remarkPlugins,
		rehypePlugins,
		recmaPlugins,
	},
};
