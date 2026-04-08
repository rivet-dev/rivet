import path from "node:path";

const mimeByExt: Record<string, string> = {
	".html": "text/html; charset=utf-8",
	".js": "text/javascript; charset=utf-8",
	".css": "text/css; charset=utf-8",
	".svg": "image/svg+xml",
	".ico": "image/x-icon",
	".json": "application/json",
	".woff2": "font/woff2",
};

export function contentType(filePath: string): string {
	const ext = path.extname(filePath);
	return mimeByExt[ext] ?? "application/octet-stream";
}
