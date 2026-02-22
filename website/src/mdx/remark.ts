import { mdxAnnotations } from "mdx-annotations";
import remarkGfm from "remark-gfm";
import { execSync } from "child_process";
import { visit } from "unist-util-visit";

// Remark plugin to add last modified time from git history
function remarkModifiedTime() {
	return function (_tree: unknown, file: { history: string[]; data: { astro?: { frontmatter?: Record<string, unknown> } } }) {
		const filepath = file.history[0];
		if (!filepath) return;

		try {
			// Use stdio: 'pipe' to suppress stderr output in CI/Docker environments
			const result = execSync(`git log -1 --pretty="format:%cI" "${filepath}"`, {
				stdio: ['pipe', 'pipe', 'pipe'],
				timeout: 5000,
			});
			const lastModified = result.toString().trim();
			if (lastModified) {
				file.data.astro = file.data.astro || {};
				file.data.astro.frontmatter = file.data.astro.frontmatter || {};
				file.data.astro.frontmatter.lastModified = lastModified;
			}
		} catch {
			// Git command may fail for new files not yet committed or in Docker builds without git history
		}
	};
}

// Preserve plain code fence metastrings (for example: ```ts registry.ts @hide) on hProperties.
// mdx-annotations only consumes JSON-like annotation blocks, so we bridge remaining metastrings
// using a neutral property that does not go through mdx-annotations' recma parser.
function remarkCodeFenceMetaToAnnotation() {
	return (tree: unknown) => {
		visit(tree, "code", (node: unknown) => {
			const code = node as {
				meta?: string | null;
				data?: {
					hProperties?: Record<string, unknown>;
				};
			};
			const meta = code.meta?.trim();
			if (!meta) return;

			const data = (code.data ??= {});
			const hProperties = (data.hProperties ??= {});
			const existingMetaString = hProperties.metastring;

			if (
				typeof existingMetaString !== "string" ||
				existingMetaString.trim().length === 0
			) {
				hProperties.metastring = meta;
			}
		});
	};
}

export const remarkPlugins = [
	mdxAnnotations.remark,
	remarkCodeFenceMetaToAnnotation,
	remarkGfm,
	remarkModifiedTime,
];
