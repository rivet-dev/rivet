import { recmaPlugins } from "./recma";
import { rehypePlugins } from "./rehype";
import { remarkPlugins } from "./remark";

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
