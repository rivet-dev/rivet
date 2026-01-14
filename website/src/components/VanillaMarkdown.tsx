import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import type { ComponentProps } from "react";

type ReactMarkdownProps = ComponentProps<typeof ReactMarkdown>;

// Minimal components for vanilla markdown rendering
const vanillaComponents: ReactMarkdownProps["components"] = {
	a: ({ href, children }) => {
		if (href?.startsWith("http")) {
			return (
				<a href={href} target="_blank" rel="noopener noreferrer">
					{children}
				</a>
			);
		}
		return <a href={href || "#"}>{children}</a>;
	},
	pre: ({ children }) => (
		<pre className="overflow-x-auto rounded-md bg-zinc-900 p-4 text-sm">
			{children}
		</pre>
	),
	code: ({ className, children }) => {
		const isInline = !className;
		if (isInline) {
			return (
				<code className="rounded bg-zinc-800 px-1.5 py-0.5 text-sm">
					{children}
				</code>
			);
		}
		return <code className={className}>{children}</code>;
	},
	table: ({ children }) => (
		<div className="overflow-x-auto">
			<table>{children}</table>
		</div>
	),
};

/**
 * Vanilla markdown renderer for standard markdown without MDX.
 * Use this for rendering GitHub-style markdown (e.g., README files).
 */
export function VanillaMarkdown({ children, content }: { children?: string; content?: string }) {
	const markdown = content || children || '';
	return (
		<ReactMarkdown remarkPlugins={[remarkGfm]} components={vanillaComponents}>
			{markdown}
		</ReactMarkdown>
	);
}
