import { mdxAnnotations } from "mdx-annotations";
import remarkGfm from "remark-gfm";
import { execSync } from "child_process";

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

export const remarkPlugins = [mdxAnnotations.remark, remarkGfm, remarkModifiedTime];
