import errImport from "./data/validation-errors.json";

interface ValidationErrors {
	GAME: TraversableObject;
	GROUP: TraversableObject;
	GAME_VERSION: TraversableObject;
	GAME_NAMESPACE: TraversableObject;
	// IDENTITY_PROFILE: TraversableObject;
	GROUP_PROFILE: TraversableObject;
	// DEV_TOKEN: TraversableObject;
	GAME_NAMESPACE_CONFIG: TraversableObject;
}

export interface ValidationPaths {
	GAME: {
		"display-name": true;
		"name-id": true;
	};
	// GROUP: string;
	GAME_VERSION: {
		"display-name": true;
	};
	GAME_NAMESPACE: {
		"display-name": true;
		"name-id": true;
	};
	// IDENTITY_PROFILE: string;
	GROUP_PROFILE: {
		"display-name": true;
		bio: true;
	};
	GAME_NAMESPACE_CONFIG: {
		"lobby-count": true;
		"max-players": true;
	};
}

// Typed JSON
export const VALIDATION_ERRORS = errImport as ValidationErrors;

export type TraversableObject = { [key: string]: TraversableObject | string };

export class TraversableError {
	path: string[];
	formattingInstructions: TraversableObject;

	constructor(formattingInstructions: TraversableObject, path: string[]) {
		this.formattingInstructions = formattingInstructions;
		this.path = path;
	}

	format(
		formatInstructions: TraversableObject = this.formattingInstructions,
	) {
		return formatError(this.path, formatInstructions);
	}
}

export class TraversableErrors {
	private errors: TraversableError[] = [];
	private formattingInstructions: TraversableObject;
	private prefix: string[] = [];

	constructor(formattingInstructions: TraversableObject, paths?: string[][]) {
		if (!formattingInstructions)
			throw new Error("Invalid formatting instructions");

		this.formattingInstructions = formattingInstructions;
		this.load(paths ?? []);
	}

	load(paths: string[][]) {
		this.errors = paths.map(
			(a) => new TraversableError(this.formattingInstructions, a),
		);
	}

	isEmpty() {
		return this.count() === 0;
	}

	count(...pathQuery: (string | number)[]) {
		// Finds all errors that start with pathQuery
		let errors = 0;
		const expandedQuery = [...this.prefix, ...pathQuery];

		// Return all errors
		if (expandedQuery.length === 0) {
			return this.errors.length;
		}

		for (const error of this.errors) {
			if (error.path.length < expandedQuery.length) continue;

			for (let i = 0, l = expandedQuery.length; i < l; i++) {
				if (error.path[i] === expandedQuery[i].toString()) {
					if (i === l - 1) {
						errors++;
						break;
					}
				} else {
					break;
				}
			}
		}

		return errors;
	}

	// Finds all errors that start with pathQuery
	find(...pathQuery: (string | number)[]) {
		const errors = [];
		const expandedQuery = [...this.prefix, ...pathQuery];

		// Return all errors
		if (expandedQuery.length === 0) {
			return Array.from(this.errors);
		}

		for (const error of this.errors) {
			if (error.path.length < expandedQuery.length) continue;

			for (let i = 0, l = expandedQuery.length; i < l; i++) {
				if (error.path[i] === expandedQuery[i].toString()) {
					if (i === l - 1) {
						errors.push(error);
						break;
					}
				} else {
					break;
				}
			}
		}

		return errors;
	}

	findFormatted(...pathQuery: (string | number)[]) {
		return this.find(...pathQuery).map((a) => a.format());
	}

	// Finds all errors that start with pathQuery (only at a depth of +1)
	findShallow(...pathQuery: (string | number)[]) {
		const errors = [];
		const expandedQuery = [...this.prefix, ...pathQuery];

		for (const error of this.errors) {
			if (error.path.length !== expandedQuery.length + 1) continue;

			if (expandedQuery.length === 0) {
				errors.push(error);
			} else {
				for (let i = 0, l = expandedQuery.length; i < l; i++) {
					if (
						error.path[i] === expandedQuery[i].toString() &&
						i === l - 1
					) {
						errors.push(error);
						break;
					}
				}
			}
		}

		return errors;
	}

	findShallowFormatted(...pathQuery: (string | number)[]) {
		return this.findShallow(...pathQuery).map((a) => a.format());
	}

	branch(...pathQuery: (string | number)[]) {
		const branch = new TraversableErrors(this.formattingInstructions);
		branch.errors = this.errors;
		branch.prefix = [...this.prefix, ...pathQuery.map((a) => a.toString())];

		return branch;
	}
}

function formatError(error: string[], traverseStart: TraversableObject) {
	let traverse: TraversableObject | string = traverseStart;

	for (const topic of error) {
		// Skip indexes
		if (!Number.isNaN(Number.parseInt(topic))) continue;
		// Skip labels
		if (topic.startsWith("*") && topic.endsWith("*")) continue;

		if (typeof traverse === "string") return traverse;

		// Check if error path exists
		if (Object.prototype.hasOwnProperty.call(traverse, topic)) {
			traverse = traverse[topic];
		}
		// Invalid error path
		else {
			console.warn("Unknown traversable error", error);
			return `${error.join(".")}`;
		}
	}

	return typeof traverse === "string" ? traverse : null;
}
