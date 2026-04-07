/**
 * Vite resolves `shiki` → this file (see `vite.config.ts`).
 *
 * A dependency pulls `shiki` at bundle entry; the real package is multi‑MB. This sample does not
 * use syntax-highlighted code blocks, so we stub `codeToHtml` to keep the client bundle small.
 */
export async function codeToHtml(
	code: string,
	_options?: { lang?: string; theme?: string } | Record<string, unknown>,
): Promise<string> {
	const esc = (s: string) =>
		s
			.replace(/&/g, "&amp;")
			.replace(/</g, "&lt;")
			.replace(/>/g, "&gt;")
			.replace(/"/g, "&quot;");
	return `<pre class="shiki-stub overflow-x-auto rounded-md border border-border bg-muted p-3 text-sm"><code>${esc(
		code,
	)}</code></pre>`;
}
