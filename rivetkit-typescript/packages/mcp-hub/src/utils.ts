const TOKEN_RATIO = 1.3;

export function estimateTokens(text: string): number {
	const words = text.split(/\s+/).filter(Boolean).length;
	return Math.max(1, Math.round(words * TOKEN_RATIO));
}

export function truncateByTokens(text: string, maxTokens: number): string {
	if (estimateTokens(text) <= maxTokens) {
		return text;
	}

	const tokens = text.split(/\s+/).filter(Boolean);
	const trimmed = tokens.slice(0, Math.max(1, Math.floor(maxTokens / TOKEN_RATIO)));
	return trimmed.join(" ") + " …";
}

export function stripMarkdown(markdown: string): string {
	return markdown
		.replace(/```[\s\S]*?```/g, " ")
		.replace(/`([^`]+)`/g, "$1")
		.replace(/!\[[^\]]*\]\([^)]+\)/g, " ")
		.replace(/\[[^\]]*\]\(([^)]+)\)/g, "$1")
		.replace(/[*_]{1,3}([^*_]+)[*_]{1,3}/g, "$1")
		.replace(/<[^>]+>/g, " ")
		.replace(/#+\s?(.+)/g, "$1")
		.replace(/>\s?/g, "")
		.replace(/\s+/g, " ")
		.trim();
}

export function encodeCursor(offset: number): string {
	return Buffer.from(JSON.stringify({ offset })).toString("base64url");
}

export function decodeCursor(cursor?: string | null): number {
	if (!cursor) return 0;
	try {
		const decoded = JSON.parse(Buffer.from(cursor, "base64url").toString());
		return typeof decoded.offset === "number" && decoded.offset >= 0 ? decoded.offset : 0;
	} catch {
		return 0;
	}
}

export function parseResourceUri(resourceUri: string): {
	pageUri: string;
	sectionAnchor?: string;
} {
	const [pageUri, fragment] = resourceUri.split("#section=");
	return {
		pageUri,
		sectionAnchor: fragment ?? undefined,
	};
}

export function safeResourceName(input: string): string {
	return input.replace(/[^a-z0-9]+/gi, "-").replace(/^-+|-+$/g, "") || "docs-resource";
}
