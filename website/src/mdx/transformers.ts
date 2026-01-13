import type { ShikiTransformer, ThemedToken } from "shiki";

interface TemplateVariable {
	variable: string;
	defaultValue?: string;
}

/**
 * Parses a template variable string and returns info about it
 * Supports: {{variable.name}} or {{variable.name:"default-value"}}
 */
function parseTemplateVariable(fullMatch: string): TemplateVariable | null {
	// Remove {{ and }}
	const content = fullMatch.slice(2, -2).trim();
	const colonIndex = content.indexOf(":");

	if (colonIndex > -1) {
		const variable = content.substring(0, colonIndex).trim();
		const defaultPart = content.substring(colonIndex + 1).trim();
		// Remove quotes from default value
		const defaultValue = defaultPart.replace(/^["']|["']$/g, "");
		return { variable, defaultValue };
	}

	return { variable: content };
}

/**
 * Shiki transformer that detects template variables in code and splits them
 * into separate tokens with data attributes so they can be made interactive.
 *
 * Template variables are in the format:
 * - {{variable.name}} - basic variable
 * - {{variable.name:"default-value"}} - variable with default value
 */
export function transformerTemplateVariables(): ShikiTransformer {
	return {
		name: "template-variables",
		tokens(tokens) {
			const newLines: ThemedToken[][] = [];

			for (const line of tokens) {
				const newTokens: ThemedToken[] = [];

				for (const token of line) {
					const splitTokens = splitTokenForTemplateVariables(token);
					newTokens.push(...splitTokens);
				}

				newLines.push(newTokens);
			}

			return newLines;
		},
	};
}

/**
 * Splits a single token that may contain template variables into multiple tokens
 */
function splitTokenForTemplateVariables(token: ThemedToken): ThemedToken[] {
	const templateVarRegex = /\{\{[^}]+\}\}/g;
	const matches = Array.from(token.content.matchAll(templateVarRegex));

	if (matches.length === 0) {
		// No template variables, return original token
		return [token];
	}

	const resultTokens: ThemedToken[] = [];
	let lastIndex = 0;

	for (const match of matches) {
		if (match.index === undefined) continue;
		const matchIndex = match.index;

		// Add text before the match as a regular token
		if (matchIndex > lastIndex) {
			resultTokens.push({
				...token,
				content: token.content.substring(lastIndex, matchIndex),
				offset: token.offset + lastIndex,
			});
		}

		// Parse the template variable
		const parsed = parseTemplateVariable(match[0]);

		if (parsed) {
			// Create a new token for the template variable with special attributes
			resultTokens.push({
				...token,
				content: parsed.defaultValue || match[0],
				offset: token.offset + matchIndex,
				htmlAttrs: {
					...(token.htmlAttrs || {}),
					"data-template-var": parsed.variable,
					...(parsed.defaultValue
						? {
								"data-template-default": parsed.defaultValue,
							}
						: {}),
				},
			});
		} else {
			// If parsing failed, keep original content
			resultTokens.push({
				...token,
				content: match[0],
				offset: token.offset + matchIndex,
			});
		}

		lastIndex = matchIndex + match[0].length;
	}

	// Add remaining text after last match
	if (lastIndex < token.content.length) {
		resultTokens.push({
			...token,
			content: token.content.substring(lastIndex),
			offset: token.offset + lastIndex,
		});
	}

	return resultTokens;
}
