// Operational transformation for plain text.
//
// An operation walks the whole document from start to end. `retain` keeps
// characters, `insert` adds text at the current position, and `delete` removes
// characters. The retained and deleted counts always add up to the length of
// the document the operation was made against.

// docs:start types
export type Component =
	| { retain: number }
	| { insert: string }
	| { delete: number };

export type Operation = Component[];
// docs:end types

// Appends a component and merges it into the previous one when both have the
// same kind, so equal edits always produce equal operations.
function push(op: Operation, component: Component): void {
	const last = op[op.length - 1];
	if ("retain" in component) {
		if (component.retain === 0) return;
		if (last && "retain" in last) {
			last.retain += component.retain;
			return;
		}
	} else if ("delete" in component) {
		if (component.delete === 0) return;
		if (last && "delete" in last) {
			last.delete += component.delete;
			return;
		}
	} else {
		if (component.insert === "") return;
		if (last && "insert" in last) {
			last.insert += component.insert;
			return;
		}
	}
	op.push({ ...component });
}

/** The length of the document an operation applies to. */
export function baseLength(op: Operation): number {
	let length = 0;
	for (const component of op) {
		if ("retain" in component) length += component.retain;
		else if ("delete" in component) length += component.delete;
	}
	return length;
}

/** True when the operation keeps every character and inserts nothing. */
export function isNoop(op: Operation): boolean {
	return op.every((component) => "retain" in component);
}

/** Throws unless `value` is a well-formed operation. Use it on client input. */
export function assertOperation(value: unknown): asserts value is Operation {
	if (!Array.isArray(value)) throw new Error("operation must be an array");
	for (const component of value) {
		const keys =
			component && typeof component === "object"
				? Object.keys(component)
				: [];
		const ok =
			keys.length === 1 &&
			(("retain" in component &&
				Number.isInteger(component.retain) &&
				component.retain > 0) ||
				("delete" in component &&
					Number.isInteger(component.delete) &&
					component.delete > 0) ||
				("insert" in component &&
					typeof component.insert === "string" &&
					component.insert.length > 0));
		if (!ok) throw new Error("invalid operation component");
	}
}

// docs:start apply
/** Applies an operation to the text it was made against. */
export function apply(text: string, op: Operation): string {
	if (baseLength(op) !== text.length) {
		throw new Error("operation does not match the document length");
	}
	let result = "";
	let index = 0;
	for (const component of op) {
		if ("retain" in component) {
			result += text.slice(index, index + component.retain);
			index += component.retain;
		} else if ("insert" in component) {
			result += component.insert;
		} else {
			index += component.delete;
		}
	}
	return result;
}
// docs:end apply

// docs:start transform
/**
 * Transforms two operations made against the same text.
 *
 * Returns `[a2, b2]` where applying `a` then `b2` gives the same text as
 * applying `b` then `a2`. When both insert at the same position, the text of
 * `a` goes first.
 */
export function transform(a: Operation, b: Operation): [Operation, Operation] {
	if (baseLength(a) !== baseLength(b)) {
		throw new Error("operations were not made against the same text");
	}
	const a2: Operation = [];
	const b2: Operation = [];
	let i = 0;
	let j = 0;
	let ca: Component | undefined = a[i++];
	let cb: Component | undefined = b[j++];

	while (ca || cb) {
		// Inserted text is new on the other side, so the other side retains it.
		if (ca && "insert" in ca) {
			push(a2, ca);
			push(b2, { retain: ca.insert.length });
			ca = a[i++];
			continue;
		}
		if (cb && "insert" in cb) {
			push(a2, { retain: cb.insert.length });
			push(b2, cb);
			cb = b[j++];
			continue;
		}
		if (!ca || !cb) {
			throw new Error("operations were not made against the same text");
		}

		// Both components now cover existing text. Consume the shorter one.
		const n = Math.min(size(ca), size(cb));
		if ("retain" in ca && "retain" in cb) {
			push(a2, { retain: n });
			push(b2, { retain: n });
		} else if ("delete" in ca && "retain" in cb) {
			push(a2, { delete: n });
		} else if ("retain" in ca && "delete" in cb) {
			push(b2, { delete: n });
		}
		// When both delete the same text, neither side deletes it again.
		ca = shrink(ca, n) ?? a[i++];
		cb = shrink(cb, n) ?? b[j++];
	}
	return [a2, b2];
}
// docs:end transform

function size(component: Component): number {
	if ("retain" in component) return component.retain;
	if ("delete" in component) return component.delete;
	return component.insert.length;
}

// Removes the first `n` characters from a retain or delete component, or
// returns undefined when the whole component was used.
function shrink(component: Component, n: number): Component | undefined {
	if ("retain" in component) {
		return component.retain > n
			? { retain: component.retain - n }
			: undefined;
	}
	if ("delete" in component) {
		return component.delete > n
			? { delete: component.delete - n }
			: undefined;
	}
	return undefined;
}

// docs:start diff
/** Builds the operation that turns `before` into `after`. */
export function diff(before: string, after: string): Operation {
	let start = 0;
	while (
		start < before.length &&
		start < after.length &&
		before[start] === after[start]
	) {
		start++;
	}
	let end = 0;
	while (
		end < before.length - start &&
		end < after.length - start &&
		before[before.length - 1 - end] === after[after.length - 1 - end]
	) {
		end++;
	}
	const op: Operation = [];
	push(op, { retain: start });
	push(op, { delete: before.length - start - end });
	push(op, { insert: after.slice(start, after.length - end) });
	push(op, { retain: end });
	return op;
}
// docs:end diff

/** Moves a caret position in the original text to the same spot after `op`. */
export function transformIndex(op: Operation, index: number): number {
	let position = 0;
	let shift = 0;
	for (const component of op) {
		if (position > index) break;
		if ("retain" in component) {
			position += component.retain;
		} else if ("insert" in component) {
			shift += component.insert.length;
		} else {
			shift -= Math.min(component.delete, index - position);
			position += component.delete;
		}
	}
	return index + shift;
}
