#!/usr/bin/env tsx
/**
 * Generate Rust boilerplate converters between two adjacent BARE schema versions.
 *
 * Each generated function maps every field/variant the two schemas have in
 * common. Anything that exists on only one side becomes `todo!()` so the human
 * implementing the migration is forced to make a decision.
 *
 *   tsx index.ts <from.bare> <to.bare> <out_dir> \
 *       [--types Type1,Type2] [--from-ns v3] [--to-ns v4]
 *
 * Two files are written: <from_ns>_to_<to_ns>.rs and <to_ns>_to_<from_ns>.rs.
 *
 * If --types is omitted, every type that exists in both schemas gets a
 * converter. If --types is provided, only those types and the types they
 * reference (transitively) are emitted.
 *
 * Types that resolve to a primitive (e.g. `type Id str`) are not given
 * converters since `vN::Id` and `vM::Id` are the same Rust type and identity
 * is a no-op.
 */
import { Command } from "commander";
import {
	Config,
	parse,
	type StructField,
	type SymbolTable,
	type Type,
	resolveAlias,
	symbols,
	withoutExtra,
} from "@bare-ts/tools";
import * as fs from "node:fs";
import * as path from "node:path";
import Handlebars from "handlebars";

type SchemaMap = Map<string, Type>;
type Schema = { table: SymbolTable; map: SchemaMap };

const TEMPLATES_DIR = path.join(import.meta.dirname ?? path.dirname(new URL(import.meta.url).pathname), "templates");

function loadTemplate(name: string): Handlebars.TemplateDelegate {
	const src = fs.readFileSync(path.join(TEMPLATES_DIR, `${name}.hbs`), "utf8");
	return Handlebars.compile(src, { noEscape: true });
}

const TEMPLATES = {
	file: loadTemplate("file"),
	struct: loadTemplate("struct"),
	union: loadTemplate("union"),
	enum: loadTemplate("enum"),
	wrapperAlias: loadTemplate("wrapper-alias"),
	todo: loadTemplate("todo"),
};

function loadSchema(p: string): Schema {
	const ast = parse(fs.readFileSync(p, "utf8"), Config({ schema: p }));
	const table = symbols(ast);
	const map: SchemaMap = new Map();
	for (const def of ast.defs) map.set(def.alias, def.type);
	return { table, map };
}

const RUST_KEYWORDS = new Set([
	"as", "break", "const", "continue", "crate", "else", "enum", "extern", "false",
	"fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut",
	"pub", "ref", "return", "self", "Self", "static", "struct", "super", "trait",
	"true", "type", "unsafe", "use", "where", "while", "async", "await", "dyn",
]);

function snake(s: string): string {
	const out = s
		.replace(/([a-z0-9])([A-Z])/g, "$1_$2")
		.replace(/([A-Z]+)([A-Z][a-z])/g, "$1_$2")
		.replace(/-/g, "_")
		.toLowerCase();
	return RUST_KEYWORDS.has(out) ? `${out}_` : out;
}

function pascalFromScreaming(s: string): string {
	return s
		.split("_")
		.map((w) => (w.length === 0 ? "" : w[0].toUpperCase() + w.slice(1).toLowerCase()))
		.join("");
}

function fnName(typeName: string, fromNs: string, toNs: string): string {
	return `convert_${snake(typeName)}_${fromNs}_to_${toNs}`;
}

function isVoidNamed(name: string, schema: Schema): boolean {
	const t = schema.map.get(name);
	return !!t && t.tag === "void";
}

function typesEqual(a: Type, b: Type): boolean {
	return JSON.stringify(withoutExtra(a)) === JSON.stringify(withoutExtra(b));
}

// A type is "trivial" if its Rust representation requires no conversion across
// the two namespaces: primitives, data, void, and aggregates whose every leaf
// resolves to those. Aliases are followed.
function isTrivial(t: Type, schema: Schema): boolean {
	switch (t.tag) {
		case "alias":
			return isTrivial(resolveAlias(t, schema.table), schema);
		case "void":
		case "data":
			return true;
		case "optional":
			return isTrivial(t.types![0]!, schema);
		case "list":
			return isTrivial(t.types![0]!, schema);
		case "map":
			return isTrivial(t.types![0]!, schema) && isTrivial(t.types![1]!, schema);
		case "struct":
		case "union":
		case "enum":
			return false;
		default:
			// All BaseTypes (u8/i32/.../bool/str).
			return true;
	}
}

// Whether the user-defined type `name` needs a converter function emitted.
// Skipped if both schemas resolve it to a trivial Rust type (e.g. `type Id str`).
function needsConverter(name: string, from: Schema, to: Schema): boolean {
	const f = from.map.get(name);
	const t = to.map.get(name);
	if (!f || !t) return false;
	if (f.tag === "void" || t.tag === "void") return false;
	if (isTrivial(f, from) && isTrivial(t, to)) return false;
	return true;
}

function transitiveSharedTypes(seed: string[], from: Schema, to: Schema): Set<string> {
	const shared = new Set<string>();
	for (const name of from.map.keys()) if (to.map.has(name)) shared.add(name);

	const out = new Set<string>();
	const stack: string[] = [];
	for (const s of seed) {
		if (!shared.has(s)) {
			console.warn(`warn: --types entry '${s}' is not present in both schemas; skipping`);
			continue;
		}
		stack.push(s);
	}

	while (stack.length > 0) {
		const name = stack.pop()!;
		if (out.has(name)) continue;
		out.add(name);

		const visit = (t: Type) => {
			if (t.tag === "alias") {
				if (shared.has(t.data) && !out.has(t.data)) stack.push(t.data);
				return;
			}
			if (t.types) for (const sub of t.types) visit(sub);
		};
		const fromType = from.map.get(name);
		const toType = to.map.get(name);
		if (fromType) visit(fromType);
		if (toType) visit(toType);
	}
	return out;
}

// Render an expression converting `expr` (typed as `t` in fromNs) into the
// target shape in toNs. `emitted` is the set of named types with a converter
// function emitted; references outside that set inline as identity when the
// alias resolves to a trivial type in both schemas, otherwise `todo!()`.
//
// Converter functions return `Result<T>`, so this returns an expression of
// type `Result<T>` when fallible (use the result with `?`) and a plain
// expression when no nested converter is involved.
//
// Returns { expr, fallible }. `fallible` indicates whether the caller must
// propagate with `?` or wrap in `Ok(...)`.
function convertExpr(
	t: Type,
	expr: string,
	fromNs: string,
	toNs: string,
	from: Schema,
	to: Schema,
	emitted: Set<string>,
): { expr: string; fallible: boolean } {
	switch (t.tag) {
		case "void":
			return { expr: "()", fallible: false };
		case "alias": {
			if (emitted.has(t.data)) {
				return {
					expr: `${fnName(t.data, fromNs, toNs)}(${expr})?`,
					fallible: true,
				};
			}
			const fromBody = from.map.get(t.data);
			const toBody = to.map.get(t.data);
			if (fromBody && toBody && isTrivial(fromBody, from) && isTrivial(toBody, to))
				return { expr, fallible: false };
			return { expr: "todo!()", fallible: false };
		}
		case "optional": {
			const inner = convertExpr(
				t.types![0]!,
				"v",
				fromNs,
				toNs,
				from,
				to,
				emitted,
			);
			if (inner.expr === "v") return { expr, fallible: false };
			if (inner.fallible) {
				const innerNoQ = inner.expr.endsWith("?")
					? inner.expr.slice(0, -1)
					: inner.expr;
				return {
					expr: `${expr}.map(|v| ${innerNoQ}).transpose()?`,
					fallible: true,
				};
			}
			return { expr: `${expr}.map(|v| ${inner.expr})`, fallible: false };
		}
		case "list": {
			const inner = convertExpr(
				t.types![0]!,
				"v",
				fromNs,
				toNs,
				from,
				to,
				emitted,
			);
			if (inner.expr === "v") return { expr, fallible: false };
			if (inner.fallible) {
				const innerNoQ = inner.expr.endsWith("?")
					? inner.expr.slice(0, -1)
					: inner.expr;
				return {
					expr: `${expr}.into_iter().map(|v| ${innerNoQ}).collect::<Result<Vec<_>>>()?`,
					fallible: true,
				};
			}
			return {
				expr: `${expr}.into_iter().map(|v| ${inner.expr}).collect()`,
				fallible: false,
			};
		}
		case "map": {
			const k = convertExpr(
				t.types![0]!,
				"k",
				fromNs,
				toNs,
				from,
				to,
				emitted,
			);
			const v = convertExpr(
				t.types![1]!,
				"v",
				fromNs,
				toNs,
				from,
				to,
				emitted,
			);
			if (k.expr === "k" && v.expr === "v")
				return { expr, fallible: false };
			const fallible = k.fallible || v.fallible;
			if (fallible) {
				return {
					expr: `${expr}.into_iter().map(|(k, v)| -> Result<_> { Ok((${k.expr}, ${v.expr})) }).collect::<Result<_>>()?`,
					fallible: true,
				};
			}
			return {
				expr: `${expr}.into_iter().map(|(k, v)| (${k.expr}, ${v.expr})).collect()`,
				fallible: false,
			};
		}
		default:
			return { expr, fallible: false };
	}
}

interface Ctx {
	fromNs: string;
	toNs: string;
	from: Schema;
	to: Schema;
	emitted: Set<string>;
}

function renderStruct(
	name: string,
	fromBody: Extract<Type, { tag: "struct" }>,
	toBody: Extract<Type, { tag: "struct" }>,
	ctx: Ctx,
): string {
	const fromMap = new Map<string, { field: StructField; type: Type }>();
	fromBody.data.forEach((f, i) =>
		fromMap.set(f.name, { field: f, type: fromBody.types![i]! }),
	);
	const fields = toBody.data.map((f, i) => {
		const toType = toBody.types![i]!;
		const rustField = snake(f.name);
		const fromEntry = fromMap.get(f.name);
		if (!fromEntry || !typesEqual(fromEntry.type, toType)) {
			return { name: rustField, expr: "todo!()" };
		}
		return {
			name: rustField,
			expr: convertExpr(
				fromEntry.type,
				`x.${rustField}`,
				ctx.fromNs,
				ctx.toNs,
				ctx.from,
				ctx.to,
				ctx.emitted,
			).expr,
		};
	});
	return TEMPLATES.struct({
		fnName: fnName(name, ctx.fromNs, ctx.toNs),
		name,
		fromNs: ctx.fromNs,
		toNs: ctx.toNs,
		fields,
	});
}

function renderUnion(
	name: string,
	fromBody: Extract<Type, { tag: "union" }>,
	toBody: Extract<Type, { tag: "union" }>,
	ctx: Ctx,
): string {
	const toVariantSet = new Set<string>();
	for (const t of toBody.types!) if (t.tag === "alias") toVariantSet.add(t.data);

	const arms: string[] = [];
	for (const variant of fromBody.types!) {
		if (variant.tag !== "alias") {
			arms.push(`// inline union variant unsupported`);
			continue;
		}
		const v = variant.data;
		const fromVoid = isVoidNamed(v, ctx.from);
		const toVoid = isVoidNamed(v, ctx.to);

		if (!toVariantSet.has(v)) {
			arms.push(
				`${ctx.fromNs}::${name}::${v}${fromVoid ? "" : "(_)"} => todo!(),`,
			);
			continue;
		}
		if (fromVoid && toVoid) {
			arms.push(
				`${ctx.fromNs}::${name}::${v} => ${ctx.toNs}::${name}::${v},`,
			);
		} else if (!fromVoid && !toVoid) {
			const inner = ctx.emitted.has(v)
				? `${fnName(v, ctx.fromNs, ctx.toNs)}(v)?`
				: (() => {
					const fromBody2 = ctx.from.map.get(v);
					const toBody2 = ctx.to.map.get(v);
					if (
						fromBody2 &&
						toBody2 &&
						isTrivial(fromBody2, ctx.from) &&
						isTrivial(toBody2, ctx.to)
					) {
						return "v";
					}
					return null;
				})();
			if (inner === null) {
				arms.push(`${ctx.fromNs}::${name}::${v}(_) => todo!(),`);
			} else {
				arms.push(
					`${ctx.fromNs}::${name}::${v}(v) => ${ctx.toNs}::${name}::${v}(${inner}),`,
				);
			}
		} else {
			arms.push(
				`${ctx.fromNs}::${name}::${v}${fromVoid ? "" : "(_)"} => todo!(),`,
			);
		}
	}
	return TEMPLATES.union({
		fnName: fnName(name, ctx.fromNs, ctx.toNs),
		name,
		fromNs: ctx.fromNs,
		toNs: ctx.toNs,
		arms,
	});
}

function renderEnum(
	name: string,
	fromBody: Extract<Type, { tag: "enum" }>,
	toBody: Extract<Type, { tag: "enum" }>,
	ctx: Ctx,
): string {
	const toValues = new Set(toBody.data.map((v) => v.name));
	const arms = fromBody.data.map((v) => {
		// @bare-ts/tools already gives the Rust-style PascalCase variant name.
		const variant = v.name;
		if (toValues.has(v.name))
			return `${ctx.fromNs}::${name}::${variant} => ${ctx.toNs}::${name}::${variant},`;
		return `${ctx.fromNs}::${name}::${variant} => todo!(),`;
	});
	return TEMPLATES.enum({
		fnName: fnName(name, ctx.fromNs, ctx.toNs),
		name,
		fromNs: ctx.fromNs,
		toNs: ctx.toNs,
		arms,
	});
}

function renderConverter(
	name: string,
	fromBody: Type,
	toBody: Type,
	ctx: Ctx,
): string {
	const renderTodo = () =>
		TEMPLATES.todo({
			fnName: fnName(name, ctx.fromNs, ctx.toNs),
			name,
			fromNs: ctx.fromNs,
			toNs: ctx.toNs,
		});

	if (fromBody.tag !== toBody.tag) return renderTodo();
	if (fromBody.tag === "struct" && toBody.tag === "struct")
		return renderStruct(name, fromBody, toBody, ctx);
	if (fromBody.tag === "union" && toBody.tag === "union")
		return renderUnion(name, fromBody, toBody, ctx);
	if (fromBody.tag === "enum" && toBody.tag === "enum")
		return renderEnum(name, fromBody, toBody, ctx);

	// Wrapper alias body (list/optional/map of a non-trivial inner type).
	if (!typesEqual(fromBody, toBody)) return renderTodo();
	const e = convertExpr(fromBody, "x", ctx.fromNs, ctx.toNs, ctx.from, ctx.to, ctx.emitted);
	return TEMPLATES.wrapperAlias({
		fnName: fnName(name, ctx.fromNs, ctx.toNs),
		name,
		fromNs: ctx.fromNs,
		toNs: ctx.toNs,
		expr: e.expr,
	});
}

function emitFile(
	fromNs: string,
	toNs: string,
	from: Schema,
	to: Schema,
	emitted: Set<string>,
	fromFile: string,
	toFile: string,
): string {
	const ctx: Ctx = { fromNs, toNs, from, to, emitted };

	const order: string[] = [];
	for (const name of to.map.keys()) {
		if (emitted.has(name) && from.map.has(name)) order.push(name);
	}
	for (const name of emitted) {
		if (!order.includes(name) && from.map.has(name) && to.map.has(name)) {
			order.push(name);
		}
	}

	const converters = order.map((name) => ({
		body: renderConverter(name, from.map.get(name)!, to.map.get(name)!, ctx).trimEnd(),
	}));

	return TEMPLATES.file({ fromNs, toNs, fromFile, toFile, converters });
}

function main() {
	const program = new Command()
		.argument("<from>", "path to old .bare schema")
		.argument("<to>", "path to new .bare schema")
		.argument("<out_dir>", "directory where the two .rs files are written")
		.option("--types <list>", "comma-separated root types to emit (default: all shared non-trivial types)")
		.option("--from-ns <ns>", "from-side Rust module name (default: from filename basename)")
		.option("--to-ns <ns>", "to-side Rust module name (default: to filename basename)")
		.parse(process.argv);

	const [fromPath, toPath, outDir] = program.processedArgs as [string, string, string];
	const opts = program.opts<{ types?: string; fromNs?: string; toNs?: string }>();
	const fromNs = opts.fromNs ?? path.basename(fromPath, ".bare");
	const toNs = opts.toNs ?? path.basename(toPath, ".bare");

	const from = loadSchema(fromPath);
	const to = loadSchema(toPath);

	let candidates: Set<string>;
	if (opts.types) {
		const seeds = opts.types.split(",").map((s) => s.trim()).filter(Boolean);
		candidates = transitiveSharedTypes(seeds, from, to);
	} else {
		candidates = new Set();
		for (const name of from.map.keys()) if (to.map.has(name)) candidates.add(name);
	}

	const emitted = new Set<string>();
	for (const name of candidates) {
		if (needsConverter(name, from, to)) emitted.add(name);
	}

	fs.mkdirSync(outDir, { recursive: true });

	const forwardPath = path.join(outDir, `${fromNs}_to_${toNs}.rs`);
	const backwardPath = path.join(outDir, `${toNs}_to_${fromNs}.rs`);
	fs.writeFileSync(
		forwardPath,
		emitFile(
			fromNs,
			toNs,
			from,
			to,
			emitted,
			path.basename(fromPath),
			path.basename(toPath),
		),
	);
	fs.writeFileSync(
		backwardPath,
		emitFile(
			toNs,
			fromNs,
			to,
			from,
			emitted,
			path.basename(toPath),
			path.basename(fromPath),
		),
	);
	console.error(`wrote ${forwardPath}`);
	console.error(`wrote ${backwardPath}`);
	console.error(`emitted ${emitted.size} type converters in each direction`);
	const skipped = candidates.size - emitted.size;
	if (skipped > 0) {
		console.error(`skipped ${skipped} trivial/void types (no converter needed)`);
	}
}

main();
