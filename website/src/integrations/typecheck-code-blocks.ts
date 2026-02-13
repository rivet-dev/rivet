import type { AstroIntegration } from "astro";
import { execSync } from "node:child_process";
import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const STAGING_DIR = join(__dirname, "../../scripts/typecheck-staging");
const SNIPPETS_DIR = join(STAGING_DIR, "snippets");
const DOCS_DIR = join(__dirname, "../content/docs");
const MONOREPO_ROOT = join(__dirname, "../../..");

// Track if rivetkit has been built this session
let rivetKitBuilt = false;

interface CodeBlock {
	code: string;
	language: string;
	lineNumber: number;
	sourceFile: string;
	nocheck: boolean;
	title?: string;
}

interface WorkspaceGroup {
	blocks: CodeBlock[];
	lineNumber: number;
	sourceFile: string;
}

interface SnippetMapping {
	snippetFile: string;
	sourceFile: string;
	lineNumber: number;
}

/**
 * Parse title from code fence annotation
 */
function parseTitleFromAnnotation(annotation: string): string | undefined {
	// Try JSON format first: {{ "title": "filename.ts" }}
	if (annotation.includes("{")) {
		try {
			// Handle {{ }} double braces by extracting inner content
			const jsonMatch = annotation.match(/\{\{?\s*([^}]+)\s*\}?\}/);
			if (jsonMatch) {
				const jsonStr = `{${jsonMatch[1]}}`;
				const parsed = JSON.parse(jsonStr);
				if (parsed.title) {
					return parsed.title;
				}
			}
		} catch {
			// Not valid JSON
		}
	}

	// Try space-separated format: filename.ts @nocheck
	const tokens = annotation.split(/\s+/);
	for (const token of tokens) {
		if (token && !token.startsWith("@") && !token.startsWith("{")) {
			// This looks like a filename
			return token;
		}
	}

	return undefined;
}

/**
 * Extract code blocks from MDX content, detecting workspace CodeGroups
 */
function extractCodeBlocks(
	content: string,
	sourceFile: string
): { blocks: CodeBlock[]; workspaces: WorkspaceGroup[] } {
	const blocks: CodeBlock[] = [];
	const workspaces: WorkspaceGroup[] = [];
	const lines = content.split("\n");

	let inCodeBlock = false;
	let inWorkspaceCodeGroup = false;
	let workspaceStartLine = 0;
	let currentWorkspaceBlocks: CodeBlock[] = [];
	let currentCode = "";
	let currentLanguage = "";
	let currentLineNumber = 0;
	let currentNocheck = false;
	let currentTitle: string | undefined;

	for (let i = 0; i < lines.length; i++) {
		const line = lines[i];
		const lineNumber = i + 1;

		// Check for CodeGroup workspace start
		if (!inCodeBlock && /<CodeGroup\s+workspace/.test(line)) {
			inWorkspaceCodeGroup = true;
			workspaceStartLine = lineNumber;
			currentWorkspaceBlocks = [];
			continue;
		}

		// Check for CodeGroup end
		if (!inCodeBlock && inWorkspaceCodeGroup && /<\/CodeGroup>/.test(line)) {
			if (currentWorkspaceBlocks.length > 0) {
				workspaces.push({
					blocks: currentWorkspaceBlocks,
					lineNumber: workspaceStartLine,
					sourceFile,
				});
			}
			inWorkspaceCodeGroup = false;
			currentWorkspaceBlocks = [];
			continue;
		}

		if (!inCodeBlock) {
			// Check for code fence start
			const match = line.match(/^```(\S*)\s*(.*)?$/);
			if (match) {
				inCodeBlock = true;
				currentLineNumber = lineNumber;
				currentCode = "";

				// Parse the annotation format
				currentLanguage = match[1] || "";
				const annotation = (match[2] || "").trim();

				currentNocheck = false;
				currentTitle = undefined;

				// Support both old JSON format and new space-separated format
				if (annotation.startsWith("{") || annotation.includes("{{")) {
					try {
						const jsonMatch = annotation.match(/\{\{?\s*([^}]+)\s*\}?\}/);
						if (jsonMatch) {
							const jsonStr = `{${jsonMatch[1]}}`;
							const parsed = JSON.parse(jsonStr);
							if (parsed.nocheck) {
								currentNocheck = true;
							}
							if (parsed.title) {
								currentTitle = parsed.title;
							}
						}
					} catch {
						// Not valid JSON, fall through to new format parsing
					}
				}

				// New format: check for @nocheck flag and title
				if (annotation.includes("@nocheck")) {
					currentNocheck = true;
				}

				// Parse title from annotation if not already set
				if (!currentTitle) {
					currentTitle = parseTitleFromAnnotation(annotation);
				}
			}
		} else {
			// Check for code fence end
			if (line.startsWith("```")) {
				inCodeBlock = false;

				// Only include TypeScript code blocks
				if (
					(currentLanguage === "ts" ||
						currentLanguage === "tsx" ||
						currentLanguage === "typescript") &&
					currentCode.trim()
				) {
					const block: CodeBlock = {
						code: currentCode,
						language: currentLanguage,
						lineNumber: currentLineNumber,
						sourceFile,
						nocheck: currentNocheck,
						title: currentTitle,
					};

					if (inWorkspaceCodeGroup) {
						currentWorkspaceBlocks.push(block);
					} else {
						blocks.push(block);
					}
				}

				currentCode = "";
				currentLanguage = "";
				currentNocheck = false;
				currentTitle = undefined;
			} else {
				currentCode += (currentCode ? "\n" : "") + line;
			}
		}
	}

	return { blocks, workspaces };
}

/**
 * Wrap code snippet for type checking
 * Adds common imports and wraps partial snippets in async IIFE
 */
function wrapCodeForTypecheck(code: string): string {
	// Check if code already has imports - if so, it's likely a complete module
	const hasImports = /^\s*import\s+/m.test(code);

	// Check if code appears to be a complete module (has exports or imports)
	const isCompleteModule =
		code.includes("export ") || hasImports || code.includes("export default");

	// Check if code is just type definitions
	const isTypeOnly =
		(code.includes("interface ") || code.includes("type ")) &&
		!code.includes("const ") &&
		!code.includes("let ") &&
		!code.includes("function ") &&
		!code.includes("class ");

	// For complete modules with imports, don't add any automatic imports
	if (isCompleteModule) {
		return code;
	}

	// For type-only definitions, don't wrap
	if (isTypeOnly) {
		return code;
	}

	// For partial snippets without imports, add common imports and wrap in async IIFE
	const imports: string[] = [];

	// Add hono import if code uses Hono
	if (code.includes("Hono") && !hasImports) {
		imports.push('import { Hono } from "hono";');
	}

	// Add zod import if code uses zod
	if (code.includes("z.") && !hasImports) {
		imports.push('import { z } from "zod";');
	}

	// Wrap in async IIFE
	const wrappedCode =
		imports.join("\n") +
		"\n\n" +
		"(async () => {\n" +
		code
			.split("\n")
			.map((line) => "  " + line)
			.join("\n") +
		"\n})();\n";

	return wrappedCode;
}

/**
 * Generate a unique filename for a snippet
 */
function generateSnippetFilename(
	sourceFile: string,
	lineNumber: number,
	language: string
): string {
	// Convert source file path to a safe filename
	const relativePath = relative(DOCS_DIR, sourceFile);
	const safeName = relativePath.replace(/[/\\]/g, "_").replace(/\.mdx?$/, "");
	const ext = language === "tsx" ? "tsx" : "ts";
	return `${safeName}_L${lineNumber}.${ext}`;
}

/**
 * Generate a directory name for a workspace group
 */
function generateWorkspaceDir(sourceFile: string, lineNumber: number): string {
	const relativePath = relative(DOCS_DIR, sourceFile);
	const safeName = relativePath.replace(/[/\\]/g, "_").replace(/\.mdx?$/, "");
	return `${safeName}_L${lineNumber}`;
}

/**
 * Parse tsc output and map errors back to source files
 */
function parseTscOutput(output: string, mappings: SnippetMapping[]): string[] {
	const errors: string[] = [];
	const lines = output.split("\n");

	for (const line of lines) {
		// Match tsc error format: file(line,col): error TS####: message
		const match = line.match(/^(.+?)\((\d+),(\d+)\): error (TS\d+): (.+)$/);
		if (match) {
			const [, file, , , errorCode, message] = match;
			// Extract the snippet path from the full path
			const snippetMatch = file.match(/snippets[/\\](.+)$/);
			const snippetFile = snippetMatch ? snippetMatch[1] : file;

			// Find the mapping for this snippet
			const mapping = mappings.find((m) => snippetFile.startsWith(m.snippetFile.replace(/\.ts$/, "")));
			if (mapping) {
				errors.push(
					`  ${mapping.sourceFile}:${mapping.lineNumber}\n    [${errorCode}] ${message}`
				);
			} else {
				errors.push(`  ${snippetFile}\n    [${errorCode}] ${message}`);
			}
		}
	}

	return errors;
}

/**
 * Recursively find all MDX files in a directory
 */
function findMdxFiles(dir: string): string[] {
	const files: string[] = [];

	if (!existsSync(dir)) {
		return files;
	}

	const entries = readdirSync(dir, { withFileTypes: true });
	for (const entry of entries) {
		const fullPath = join(dir, entry.name);
		if (entry.isDirectory()) {
			files.push(...findMdxFiles(fullPath));
		} else if (entry.name.endsWith(".mdx") || entry.name.endsWith(".md")) {
			files.push(fullPath);
		}
	}

	return files;
}

export function typecheckCodeBlocks(): AstroIntegration {
	return {
		name: "typecheck-code-blocks",
		hooks: {
			"astro:config:setup": async ({ logger }) => {
				logger.info("Type checking documentation code blocks...");

				// Clean and create snippets directory
				if (existsSync(SNIPPETS_DIR)) {
					rmSync(SNIPPETS_DIR, { recursive: true });
				}
				mkdirSync(SNIPPETS_DIR, { recursive: true });

				// Find all MDX files
				const mdxFiles = findMdxFiles(DOCS_DIR);
				logger.info(`Found ${mdxFiles.length} documentation files`);

				// Extract code blocks from all files
				const allBlocks: CodeBlock[] = [];
				const allWorkspaces: WorkspaceGroup[] = [];

				for (const file of mdxFiles) {
					const content = readFileSync(file, "utf-8");
					const { blocks, workspaces } = extractCodeBlocks(content, file);
					allBlocks.push(...blocks);
					allWorkspaces.push(...workspaces);
				}

				// Filter out nocheck blocks
				const blocksToCheck = allBlocks.filter((block) => !block.nocheck);
				const workspacesToCheck = allWorkspaces.filter((ws) =>
					ws.blocks.some((block) => !block.nocheck)
				);

				const totalBlocks =
					allBlocks.length +
					allWorkspaces.reduce((sum, ws) => sum + ws.blocks.length, 0);
				const totalToCheck =
					blocksToCheck.length +
					workspacesToCheck.reduce(
						(sum, ws) => sum + ws.blocks.filter((b) => !b.nocheck).length,
						0
					);

				logger.info(
					`Found ${totalBlocks} TypeScript code blocks, ${totalToCheck} to check (${allWorkspaces.length} workspace groups)`
				);

				if (totalToCheck === 0) {
					logger.info("No code blocks to type check");
					return;
				}

				// Write snippets and build mapping
				const mappings: SnippetMapping[] = [];

				// Write individual code blocks
				for (const block of blocksToCheck) {
					const snippetFile = generateSnippetFilename(
						block.sourceFile,
						block.lineNumber,
						block.language
					);
					const wrappedCode = wrapCodeForTypecheck(block.code);

					writeFileSync(join(SNIPPETS_DIR, snippetFile), wrappedCode);
					mappings.push({
						snippetFile,
						sourceFile: relative(process.cwd(), block.sourceFile),
						lineNumber: block.lineNumber,
					});
				}

				// Write workspace groups to subdirectories
				for (const workspace of workspacesToCheck) {
					const workspaceDir = generateWorkspaceDir(
						workspace.sourceFile,
						workspace.lineNumber
					);
					const workspacePath = join(SNIPPETS_DIR, workspaceDir);
					mkdirSync(workspacePath, { recursive: true });

					for (const block of workspace.blocks) {
						if (block.nocheck) continue;

						// Use title as filename, or fall back to line number
						const filename = block.title || `file_L${block.lineNumber}.ts`;
						// Ensure .ts extension
						const finalFilename = filename.endsWith(".ts") || filename.endsWith(".tsx")
							? filename
							: `${filename}.ts`;

						writeFileSync(join(workspacePath, finalFilename), block.code);
					}

					mappings.push({
						snippetFile: workspaceDir,
						sourceFile: relative(process.cwd(), workspace.sourceFile),
						lineNumber: workspace.lineNumber,
					});
				}

				// Build rivetkit packages if not already built this session
				if (!rivetKitBuilt) {
					logger.info("Building rivetkit packages...");
					try {
						execSync("pnpm build -F rivetkit -F @rivetkit/react -F @rivetkit/cloudflare-workers", {
							cwd: MONOREPO_ROOT,
							encoding: "utf-8",
							stdio: ["pipe", "pipe", "pipe"],
						});
						rivetKitBuilt = true;
						logger.info("rivetkit packages built successfully");
					} catch (buildError: unknown) {
						const execError = buildError as { stdout?: string; stderr?: string };
						const output = execError.stdout || execError.stderr || "";
						logger.error("Failed to build rivetkit packages:");
						console.error(output);
						throw new Error("Failed to build rivetkit packages");
					}
				}

				// Run tsc
				try {
					execSync("npx tsc --noEmit", {
						cwd: STAGING_DIR,
						encoding: "utf-8",
						stdio: ["pipe", "pipe", "pipe"],
					});
					logger.info("All code blocks passed type checking");
				} catch (error: unknown) {
					const execError = error as { stdout?: string; stderr?: string };
					const output = execError.stdout || execError.stderr || "";
					const errors = parseTscOutput(output, mappings);

					if (errors.length > 0) {
						logger.error("Type errors found in documentation code blocks:");
						for (const err of errors) {
							console.error(err);
						}
						throw new Error(
							`Type checking failed with ${errors.length} error(s) in documentation code blocks`
						);
					} else {
						// If we couldn't parse errors, show raw output
						logger.error("Type checking failed:");
						console.error(output);
						throw new Error("Type checking failed");
					}
				}
			},
		},
	};
}
