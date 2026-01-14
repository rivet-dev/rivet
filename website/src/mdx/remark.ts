import { mdxAnnotations } from "mdx-annotations";
import remarkGfm from "remark-gfm";
import { execSync } from "child_process";

// Remark plugin to add last modified time from git history
function remarkModifiedTime() {
	return function (_tree: unknown, file: { history: string[]; data: { astro?: { frontmatter?: Record<string, unknown> } } }) {
		const filepath = file.history[0];
		if (!filepath) return;

		try {
			const result = execSync(`git log -1 --pretty="format:%cI" "${filepath}"`);
			const lastModified = result.toString().trim();
			if (lastModified) {
				file.data.astro = file.data.astro || {};
				file.data.astro.frontmatter = file.data.astro.frontmatter || {};
				file.data.astro.frontmatter.lastModified = lastModified;
			}
		} catch {
			// Git command may fail for new files not yet committed
		}
	};
}

export const remarkPlugins = [mdxAnnotations.remark, remarkGfm, remarkModifiedTime];
