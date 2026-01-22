import { describe, expect, test } from "vitest";
import {
	decodeCursor,
	encodeCursor,
	estimateTokens,
	parseResourceUri,
	safeResourceName,
	stripMarkdown,
	truncateByTokens,
} from "./utils";

describe("estimateTokens", () => {
	test("returns at least 1 for empty string", () => {
		expect(estimateTokens("")).toBe(1);
	});

	test("estimates tokens based on word count", () => {
		expect(estimateTokens("hello world")).toBe(3); // 2 words * 1.3 = 2.6, rounded to 3
	});

	test("handles multiple spaces", () => {
		expect(estimateTokens("hello   world")).toBe(3);
	});

	test("handles longer text", () => {
		const text = "one two three four five six seven eight nine ten";
		expect(estimateTokens(text)).toBe(13); // 10 words * 1.3 = 13
	});
});

describe("truncateByTokens", () => {
	test("returns full text if under token limit", () => {
		const text = "hello world";
		expect(truncateByTokens(text, 100)).toBe(text);
	});

	test("truncates text that exceeds token limit", () => {
		const text = "one two three four five six seven eight nine ten";
		const result = truncateByTokens(text, 5);
		expect(result).toContain("…");
		expect(result.split(" ").length).toBeLessThan(text.split(" ").length);
	});

	test("handles edge case with very small token limit", () => {
		const text = "hello world foo bar";
		const result = truncateByTokens(text, 1);
		expect(result).toBe("hello …");
	});
});

describe("stripMarkdown", () => {
	test("removes code blocks", () => {
		const input = "Before ```code here``` After";
		expect(stripMarkdown(input)).toBe("Before After");
	});

	test("removes inline code", () => {
		const input = "Use `command` here";
		expect(stripMarkdown(input)).toBe("Use command here");
	});

	test("removes images", () => {
		const input = "Text ![alt](image.png) more";
		expect(stripMarkdown(input)).toBe("Text more");
	});

	test("extracts link URLs", () => {
		const input = "Visit [Google](https://google.com) now";
		expect(stripMarkdown(input)).toBe("Visit https://google.com now");
	});

	test("removes bold and italic markers", () => {
		const input = "This is **bold** and *italic* and ***both***";
		expect(stripMarkdown(input)).toBe("This is bold and italic and both");
	});

	test("removes HTML tags", () => {
		const input = "Hello <div>content</div> world";
		expect(stripMarkdown(input)).toBe("Hello content world");
	});

	test("removes heading markers", () => {
		const input = "## Heading Title";
		expect(stripMarkdown(input)).toBe("Heading Title");
	});

	test("removes blockquote markers", () => {
		const input = "> quoted text";
		expect(stripMarkdown(input)).toBe("quoted text");
	});

	test("normalizes whitespace", () => {
		const input = "multiple   spaces\n\nand\nnewlines";
		expect(stripMarkdown(input)).toBe("multiple spaces and newlines");
	});
});

describe("encodeCursor / decodeCursor", () => {
	test("encodes and decodes offset correctly", () => {
		const offset = 42;
		const encoded = encodeCursor(offset);
		expect(decodeCursor(encoded)).toBe(offset);
	});

	test("returns 0 for null or undefined cursor", () => {
		expect(decodeCursor(null)).toBe(0);
		expect(decodeCursor(undefined)).toBe(0);
	});

	test("returns 0 for invalid cursor", () => {
		expect(decodeCursor("invalid")).toBe(0);
	});

	test("returns 0 for negative offset in cursor", () => {
		const encoded = Buffer.from(JSON.stringify({ offset: -5 })).toString(
			"base64url",
		);
		expect(decodeCursor(encoded)).toBe(0);
	});

	test("returns 0 for non-numeric offset", () => {
		const encoded = Buffer.from(JSON.stringify({ offset: "abc" })).toString(
			"base64url",
		);
		expect(decodeCursor(encoded)).toBe(0);
	});

	test("handles offset of 0", () => {
		const encoded = encodeCursor(0);
		expect(decodeCursor(encoded)).toBe(0);
	});

	test("handles large offsets", () => {
		const offset = 10000;
		const encoded = encodeCursor(offset);
		expect(decodeCursor(encoded)).toBe(offset);
	});
});

describe("parseResourceUri", () => {
	test("parses URI without section", () => {
		const result = parseResourceUri("docs://page/getting-started");
		expect(result).toEqual({
			pageUri: "docs://page/getting-started",
			sectionAnchor: undefined,
		});
	});

	test("parses URI with section anchor", () => {
		const result = parseResourceUri(
			"docs://page/getting-started#section=installation",
		);
		expect(result).toEqual({
			pageUri: "docs://page/getting-started",
			sectionAnchor: "installation",
		});
	});

	test("handles URI with complex section anchor", () => {
		const result = parseResourceUri(
			"docs://page/api-reference#section=create-actor-method",
		);
		expect(result).toEqual({
			pageUri: "docs://page/api-reference",
			sectionAnchor: "create-actor-method",
		});
	});

	test("handles URI with no path", () => {
		const result = parseResourceUri("docs://page");
		expect(result).toEqual({
			pageUri: "docs://page",
			sectionAnchor: undefined,
		});
	});
});

describe("safeResourceName", () => {
	test("keeps alphanumeric characters", () => {
		expect(safeResourceName("hello123")).toBe("hello123");
	});

	test("replaces special characters with hyphens", () => {
		expect(safeResourceName("hello/world")).toBe("hello-world");
	});

	test("replaces multiple special characters with single hyphen", () => {
		expect(safeResourceName("hello//world")).toBe("hello-world");
	});

	test("removes leading hyphens", () => {
		expect(safeResourceName("/hello")).toBe("hello");
	});

	test("removes trailing hyphens", () => {
		expect(safeResourceName("hello/")).toBe("hello");
	});

	test("returns fallback for empty result", () => {
		expect(safeResourceName("///")).toBe("docs-resource");
	});

	test("handles mixed case", () => {
		expect(safeResourceName("Hello-World")).toBe("Hello-World");
	});

	test("handles complex paths", () => {
		expect(safeResourceName("actors/getting-started/installation")).toBe(
			"actors-getting-started-installation",
		);
	});
});
