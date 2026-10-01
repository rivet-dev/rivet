#!/usr/bin/env node
// Generates the HTTP API reference bundle at docs/api from the OpenAPI specs
// checked into this repo.
//
//   node scripts/docs/gen-api-reference.mjs          # write docs/api/content/**/*.mdx + sidebar.json
//   node scripts/docs/gen-api-reference.mjs --check  # exit 1 if the committed output is stale
//
// Inputs:
//   docs/api/endpoints.json          which operations to publish, grouping, prose, examples
//   engine/artifacts/openapi.json    control plane served at api.rivet.dev
//   rivetkit-openapi/openapi.json    actor gateway served at api.rivet.dev/gateway
//   rivetkit-asyncapi/asyncapi.json  WebSocket connection protocol on /gateway/{actor}/connect
//   engine/artifacts/errors/*.json   every error group/code/message, written at compile time
//
// Hand-written pages live directly under docs/api/content. Generated pages live
// in the per-group directories named by endpoints.json, which this script owns
// and rewrites in full. The error codes are not a page: they are written to the
// data file named by `errorCodes.output`, which the website renders on the
// hand-written error-codes page through its ErrorCodes component.

import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const BUNDLE = join(ROOT, "docs", "api");
const CONTENT = join(BUNDLE, "content");
const CONFIG_PATH = join(BUNDLE, "endpoints.json");
const SIDEBAR_PATH = join(BUNDLE, "sidebar.json");
const SELF = relative(ROOT, fileURLToPath(import.meta.url));

const checkMode = process.argv.includes("--check");

const config = readJson(CONFIG_PATH);
const specs = Object.fromEntries(
	Object.entries(config.specs).map(([id, spec]) => [id, { ...spec, doc: readJson(join(ROOT, spec.file)) }]),
);

/** @type {Map<string, string>} generated file path (relative to ROOT) -> contents */
const outputs = new Map();
const sidebar = [];

for (const group of config.groups) sidebar.push(buildGroup(group));

/**
 * Renders a group's pages and returns its sidebar entry. Sidebar order is
 * endpoints, then protocol pages, then hand-written pages, then `children`
 * groups as collapsible sub-sections. Any entry can name the generated slug it
 * should follow with `after`.
 */
function buildGroup(group) {
	const generated = [...(group.endpoints ?? []), ...(group.protocol ?? [])];
	if (generated.length === 0 && !group.children) return { title: group.title, pages: group.pages };
	const pages = [];
	// The website renders these badges as colored method pills in the sidebar.
	for (const endpoint of group.endpoints ?? []) {
		const page = renderEndpoint(group, endpoint);
		outputs.set(relative(ROOT, join(CONTENT, group.dir, `${endpoint.slug}.mdx`)), page.mdx);
		pages.push({ title: page.title, href: page.href, badge: page.badge, after: endpoint.after });
	}
	for (const protocolPage of group.protocol ?? []) {
		const page = renderProtocolPage(group, protocolPage);
		outputs.set(relative(ROOT, join(CONTENT, group.dir, `${protocolPage.slug}.mdx`)), page.mdx);
		pages.push({ title: page.title, href: page.href, badge: "WS", after: protocolPage.after });
	}
	pages.push(...(group.pages ?? []));
	const ordered = orderPages(group, pages);
	for (const child of group.children ?? []) ordered.push({ ...buildGroup(child), collapsible: true });
	return { title: group.title, icon: group.icon, pages: ordered };
}

if (config.errorCodes) {
	outputs.set(relative(ROOT, join(BUNDLE, config.errorCodes.output)), buildErrorRegistry(config.errorCodes));
}

outputs.set(relative(ROOT, SIDEBAR_PATH), `${JSON.stringify({ docs: sidebar }, null, "\t")}\n`);

const allGroups = config.groups.flatMap((g) => [g, ...(g.children ?? [])]);
const generatedDirs = allGroups.filter((g) => g.endpoints || g.protocol).map((g) => join(CONTENT, g.dir));

if (checkMode) {
	const stale = [];
	for (const [path, expected] of outputs) {
		const abs = join(ROOT, path);
		if (!existsSync(abs) || readFileSync(abs, "utf8") !== expected) stale.push(path);
	}
	for (const dir of generatedDirs) {
		if (!existsSync(dir)) continue;
		for (const file of readdirSync(dir)) {
			const rel = relative(ROOT, join(dir, file));
			if (!outputs.has(rel)) stale.push(`${rel} (orphaned)`);
		}
	}
	if (stale.length > 0) {
		console.error(`docs/api is out of date. Run \`node ${SELF}\` and commit the result.`);
		for (const path of stale) console.error(`  ${path}`);
		process.exit(1);
	}
	console.log(`docs/api is up to date (${outputs.size} files)`);
} else {
	for (const dir of generatedDirs) rmSync(dir, { recursive: true, force: true });
	for (const [path, contents] of outputs) {
		const abs = join(ROOT, path);
		mkdirSync(dirname(abs), { recursive: true });
		writeFileSync(abs, contents);
	}
	console.log(`wrote ${outputs.size} files under docs/api`);
}

/**
 * Applies ordering hints. `first: true` moves an entry to the top of the group,
 * for a concept page that endpoint pages build on. `after` names the generated
 * slug an entry should directly follow. Entries without a hint keep their order.
 */
function orderPages(group, pages) {
	const ordered = [
		...pages.filter((p) => p.first).map(({ first: _, ...page }) => page),
		...pages.filter((p) => !p.first && !p.after).map(({ after: _, ...page }) => page),
	];
	for (const { after, ...page } of pages.filter((p) => p.after && !p.first)) {
		const index = ordered.findIndex((p) => p.href === `/docs/api/${group.dir}/${after}`);
		if (index === -1) throw new Error(`${page.href}: after '${after}' is not a generated page in ${group.title}`);
		ordered.splice(index + 1, 0, page);
	}
	return ordered;
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

function renderEndpoint(group, endpoint) {
	const spec = specs[endpoint.spec];
	if (!spec) throw new Error(`${endpoint.path}: unknown spec '${endpoint.spec}'`);
	const pathItem = spec.doc.paths[endpoint.path];
	if (!pathItem) throw new Error(`${endpoint.path}: not found in ${spec.file}`);

	const methods = endpoint.method === "any" ? Object.keys(pathItem).filter(isHttpMethod) : [endpoint.method];
	const operation = pathItem[methods[0]];
	if (!operation) throw new Error(`${endpoint.method.toUpperCase()} ${endpoint.path}: not found in ${spec.file}`);

	const methodLabel = endpoint.method === "any" ? "ANY" : endpoint.method.toUpperCase();
	const href = `/docs/api/${group.dir}/${endpoint.slug}`;
	const resolveRef = (schema) => deref(spec.doc, schema);

	const parameters = [...(pathItem.parameters ?? []), ...(operation.parameters ?? [])].map((p) => resolveRef(p));
	// Precedence: per-endpoint override, curated spec-wide override, then the spec's own text.
	const describeParam = (p) => endpoint.parameters?.[p.name] ?? spec.parameters?.[p.name] ?? p.description ?? "";
	const pathParams = parameters.filter((p) => p.in === "path");
	const selectorParams = parameters.filter((p) => p.in === "query" && p.name.startsWith("rvt-"));
	const queryParams = parameters.filter((p) => p.in === "query" && !p.name.startsWith("rvt-"));
	const headerParams = parameters.filter((p) => p.in === "header");
	const actorSelector = pathParams.find((p) => p["x-rivet-actor-selector"]);

	// Endpoints may name one of the spec's `authProfiles` (for example the
	// inspector token) instead of the spec's default authentication.
	const auth = endpoint.auth ? spec.authProfiles?.[endpoint.auth] : spec.auth;
	if (!auth) throw new Error(`${endpoint.path}: unknown auth profile '${endpoint.auth}' for spec '${endpoint.spec}'`);

	const requestBody = operation.requestBody ? resolveRef(operation.requestBody) : undefined;
	const requestSchema = requestBody?.content?.["application/json"]?.schema;
	// Body and response field overrides use the dotted path shown in the table, e.g. `grants[].resource`.
	const describeField = (path, prop) =>
		endpoint.parameters?.[path] ?? spec.parameters?.[path] ?? prop.description ?? "";
	const requestFields = requestSchema
		? flattenSchema(resolveRef(requestSchema), resolveRef, describeField)
		: [];

	const lines = [];
	lines.push("---");
	lines.push(`title: ${JSON.stringify(endpoint.title)}`);
	lines.push(`description: ${JSON.stringify(endpoint.description)}`);
	lines.push("---");
	lines.push("");
	lines.push(
		`{/* Generated by ${SELF} from ${spec.file} and docs/api/endpoints.json. Do not edit by hand. */}`,
	);
	// A group `notice` (for example a stability caveat) opens every page in the group.
	if (group.notice) {
		lines.push("");
		lines.push("<Note>");
		lines.push(mdxText(group.notice.trim()));
		lines.push("</Note>");
	}
	// The site renders the frontmatter description under the title, so the body
	// starts with the notes (if any) rather than repeating it.
	if (endpoint.notes) {
		lines.push("");
		lines.push(mdxText(endpoint.notes.trim()));
	}
	lines.push("");
	lines.push("```http");
	lines.push(`${methodLabel} ${config.baseUrl}${endpoint.path}`);
	lines.push("```");
	if (methods.length > 1) {
		lines.push("");
		lines.push(`Accepts ${methods.map((m) => `\`${m.toUpperCase()}\``).join(", ")}.`);
	}

	// Examples
	lines.push("");
	lines.push("## Examples");
	lines.push("");
	lines.push("<CodeGroup>");
	lines.push("");
	lines.push("```bash cURL");
	lines.push(
		renderCurl(auth, endpoint, endpoint.examples?.method ?? methods[0], pathParams, queryParams, requestSchema, resolveRef),
	);
	lines.push("```");
	lines.push("");
	if (endpoint.typescript) {
		lines.push(codeSnippet(endpoint.slug, endpoint.typescript, "TypeScript"));
		lines.push("");
	}
	lines.push("</CodeGroup>");

	// Authentication
	lines.push("");
	lines.push("## Authentication");
	lines.push("");
	lines.push(auth.description.trim());

	// Parameters
	if (pathParams.length > 0) {
		lines.push("");
		lines.push("## Path Parameters");
		lines.push("");
		lines.push(paramTable(pathParams, describeParam, resolveRef));
	}
	if (queryParams.length > 0) {
		lines.push("");
		lines.push("## Query Parameters");
		lines.push("");
		lines.push(paramTable(queryParams, describeParam, resolveRef));
	}
	if (headerParams.length > 0) {
		lines.push("");
		lines.push("## Headers");
		lines.push("");
		lines.push(paramTable(headerParams, describeParam, resolveRef));
	}
	if (actorSelector && selectorParams.length > 0) {
		lines.push("");
		lines.push("## Actor Selector");
		lines.push("");
		lines.push(
			"`{actor}` accepts an actor ID, an actor ID with an inline routing token (`{actor_id}@{token}`), or an actor name combined with the `rvt-*` query parameters below. See [Actor Routing](/docs/api/actor-routing) for the full rules.",
		);
		lines.push("");
		lines.push('<Accordion title="rvt-* query parameters">');
		lines.push("");
		lines.push(paramTable(selectorParams, describeParam, resolveRef));
		lines.push("");
		lines.push("</Accordion>");
	}

	// Request body
	if (requestSchema) {
		lines.push("");
		lines.push("## Request Body");
		lines.push("");
		lines.push(`Content type: \`application/json\`${requestBody.required ? "" : " (optional)"}.`);
		if (requestFields.length > 0) {
			lines.push("");
			lines.push(fieldTable(requestFields, { showRequired: true }));
		} else {
			lines.push("");
			lines.push("Send an empty JSON object: `{}`.");
		}
	}

	// Responses
	const responses = Object.entries(operation.responses ?? {});
	if (responses.length > 0) {
		lines.push("");
		lines.push("## Responses");
		for (const [status, rawResponse] of responses) {
			const response = resolveRef(rawResponse);
			lines.push("");
			lines.push(`### ${status}`);
			const description = endpoint.responses?.[status] ?? response.description;
			if (description) {
				lines.push("");
				lines.push(mdxText(description));
			}
			const content = response.content ?? {};
			for (const [contentType, media] of Object.entries(content)) {
				if (!media.schema) continue;
				const schema = resolveRef(media.schema);
				if (contentType === "application/json") {
					const fields = flattenSchema(schema, resolveRef, describeField);
					if (fields.length > 0) {
						lines.push("");
						lines.push(fieldTable(fields, { showRequired: false }));
					}
					const example = endpoint.examples?.response ?? exampleFor(schema, resolveRef, { name: "", optional: true });
					if (example !== undefined) {
						lines.push("");
						lines.push("```json");
						lines.push(JSON.stringify(example, null, 2));
						lines.push("```");
					}
				} else {
					lines.push("");
					lines.push(`Content type: \`${contentType}\`.`);
				}
			}
		}
	}

	if (endpoint.related?.length) {
		lines.push("");
		lines.push("## Related");
		lines.push("");
		for (const link of endpoint.related) lines.push(`- [${link.title}](${link.href})`);
	}

	lines.push("");
	return { title: endpoint.title, href, badge: methodLabel, mdx: lines.join("\n") };
}

/**
 * Renders the WebSocket connection protocol as one page. The page opens with
 * the upgrade (URL, subprotocols, envelope), then one section per concern
 * (handshake, actions, events, errors). Each section documents its messages
 * from the AsyncAPI spec: direction, fields, and the JSON envelope, next to a
 * raw WebSocket and TypeScript example.
 */
function renderProtocolPage(group, page) {
	const spec = specs[group.spec];
	if (!spec) throw new Error(`${page.slug}: unknown spec '${group.spec}'`);
	const doc = spec.doc;
	const resolveRef = (schema) => deref(doc, schema);
	const href = `/docs/api/${group.dir}/${page.slug}`;

	// Direction comes from which top-level message (ToClient / ToServer) lists the tag.
	const direction = new Map();
	for (const [messageName, label] of [
		["ToServer", "Client to server"],
		["ToClient", "Server to client"],
	]) {
		const variants = doc.components.messages[messageName]?.payload?.properties?.body?.oneOf ?? [];
		for (const variant of variants) {
			const tag = variant.properties?.tag?.const;
			if (tag) direction.set(tag, label);
		}
	}
	if (direction.size === 0) throw new Error(`${spec.file}: could not read message tags from ToServer/ToClient`);

	const lines = [];
	lines.push("---");
	lines.push(`title: ${JSON.stringify(page.title)}`);
	lines.push(`description: ${JSON.stringify(page.description)}`);
	lines.push("---");
	lines.push("");
	lines.push(`{/* Generated by ${SELF} from ${spec.file} and docs/api/endpoints.json. Do not edit by hand. */}`);
	if (page.notes) {
		lines.push("");
		lines.push(mdxText(page.notes.trim()));
	}
	if (page.url) {
		lines.push("");
		lines.push("```http");
		lines.push(`GET ${page.url}`);
		lines.push("```");
	}

	const documented = new Set();
	for (const section of page.sections) {
		lines.push("");
		lines.push(`## ${section.title}`);
		if (section.notes) {
			lines.push("");
			lines.push(mdxText(section.notes.trim()));
		}
		if (section.snippets?.length) {
			lines.push("");
			lines.push("<CodeGroup>");
			lines.push("");
			for (const snippet of section.snippets) {
				lines.push(codeSnippet(page.slug, snippet.file, snippet.title));
				lines.push("");
			}
			lines.push("</CodeGroup>");
		}

		for (const name of section.messages ?? []) {
			const schema = doc.components.schemas[name];
			if (!schema) throw new Error(`${page.slug}: message '${name}' not found in ${spec.file}`);
			const from = direction.get(name);
			if (!from) throw new Error(`${page.slug}: message '${name}' is not listed under ToServer or ToClient`);
			documented.add(name);

			lines.push("");
			lines.push(`### ${name}`);
			lines.push("");
			lines.push(`**${from}.** ${mdxText(page.messageNotes?.[name] ?? schema.description ?? "")}`.trim());

			const describeField = (path, prop) => page.fields?.[`${name}.${path}`] ?? prop.description ?? "";
			const fields = flattenSchema(resolveRef(schema), resolveRef, describeField);
			if (fields.length > 0) {
				lines.push("");
				lines.push(fieldTable(fields, { showRequired: true }));
			}

			const val = page.examples?.[name] ?? exampleFor(resolveRef(schema), resolveRef, { name: "", optional: true });
			lines.push("");
			lines.push("```json");
			lines.push(JSON.stringify({ body: { tag: name, val } }, null, 2));
			lines.push("```");
		}
	}
	// Every message the spec can put on the wire must be documented somewhere on the page.
	const missing = [...direction.keys()].filter((name) => !documented.has(name));
	if (missing.length > 0) throw new Error(`${page.slug}: messages not documented in any section: ${missing.join(", ")}`);

	if (page.related?.length) {
		lines.push("");
		lines.push("## Related");
		lines.push("");
		for (const link of page.related) lines.push(`- [${link.title}](${link.href})`);
	}

	lines.push("");
	return { title: page.title, href, mdx: lines.join("\n") };
}

/**
 * Builds the error registry from engine/artifacts/errors. The website renders
 * it on the Error Codes page (src/components/docs/ErrorCodes.astro), so this
 * is data only; the page prose is hand-written in docs/api/content.
 *
 * HTTP statuses come from the `match (group, code)` tables in the Rust sources
 * listed in `statusSources`. A code with no explicit arm has an empty
 * `statuses` list and the page explains the default that applies.
 */
function buildErrorRegistry(cfg) {
	const dir = join(ROOT, cfg.artifacts);
	const exclude = new Set(cfg.exclude ?? []);
	/** @type {Map<string, {code: string, message: string}[]>} */
	const byGroup = new Map();
	for (const file of readdirSync(dir).sort()) {
		if (!file.endsWith(".json")) continue;
		const entry = readJson(join(dir, file));
		if (exclude.has(entry.group)) continue;
		if (!byGroup.has(entry.group)) byGroup.set(entry.group, []);
		byGroup.get(entry.group).push({ code: entry.code, message: entry.message });
	}
	if (byGroup.size === 0) throw new Error(`${cfg.artifacts}: no error artifacts found`);
	// A described group with no artifacts (`user`, whose codes are supplied by
	// actor code) is still listed so the page can explain it.
	for (const group of Object.keys(cfg.groups ?? {})) {
		if (!byGroup.has(group)) byGroup.set(group, []);
	}

	const statuses = parseStatusTables(cfg.statusSources);
	const statusesFor = (group, errorCode) => {
		/** @type {Map<number, Set<string>>} status -> surfaces that return it */
		const found = new Map();
		for (const source of statuses) {
			const status = source.codes.get(`${group}.${errorCode}`) ?? source.groups.get(group);
			if (status === undefined) continue;
			if (!found.has(status)) found.set(status, new Set());
			found.get(status).add(source.label);
		}
		return [...found]
			.sort(([a], [b]) => a - b)
			.map(([status, surfaces]) => ({ status, surfaces: [...surfaces].sort() }));
	};

	const groups = [...byGroup]
		.sort(([a], [b]) => a.localeCompare(b))
		.map(([name, entries]) => ({
			name,
			description: cfg.groups?.[name] ?? null,
			errors: entries
				.sort((a, b) => a.code.localeCompare(b.code))
				.map((e) => ({ code: e.code, message: e.message, statuses: statusesFor(name, e.code) })),
		}));

	return `${JSON.stringify(
		{
			$comment: `Generated by ${SELF} from ${cfg.artifacts} and docs/api/endpoints.json. Do not edit by hand.`,
			groups,
		},
		null,
		"\t",
	)}\n`;
}

/**
 * Extracts `("group", "code" | "other") => StatusCode::X` and `=> Some(N)` arms
 * from Rust `match (group, code)` tables. A `_` code applies to the whole group.
 */
function parseStatusTables(sources) {
	const statusCodes = {
		BAD_REQUEST: 400,
		UNAUTHORIZED: 401,
		FORBIDDEN: 403,
		NOT_FOUND: 404,
		METHOD_NOT_ALLOWED: 405,
		REQUEST_TIMEOUT: 408,
		PAYLOAD_TOO_LARGE: 413,
		TOO_MANY_REQUESTS: 429,
		INTERNAL_SERVER_ERROR: 500,
		BAD_GATEWAY: 502,
		SERVICE_UNAVAILABLE: 503,
		GATEWAY_TIMEOUT: 504,
	};
	const armPattern =
		/\(\s*"([a-z_]+)"\s*,\s*((?:"[a-z_]+"\s*(?:\|\s*)?)+|_)\s*\)\s*=>\s*(?:StatusCode::([A-Z_]+)|Some\((\d+)\))/g;
	return sources.map((source) => {
		const text = readFileSync(join(ROOT, source.file), "utf8");
		const codes = new Map();
		const groups = new Map();
		let count = 0;
		for (const match of text.matchAll(armPattern)) {
			const [, group, codeList, statusName, statusNumber] = match;
			const status = statusNumber ? Number(statusNumber) : statusCodes[statusName];
			if (status === undefined) throw new Error(`${source.file}: unknown StatusCode::${statusName}`);
			count++;
			if (codeList === "_") {
				groups.set(group, status);
				continue;
			}
			for (const quoted of codeList.match(/"[a-z_]+"/g)) {
				codes.set(`${group}.${quoted.slice(1, -1)}`, status);
			}
		}
		if (count === 0) throw new Error(`${source.file}: found no (group, code) => status arms`);
		return { label: source.label, codes, groups };
	});
}

function codeSnippet(slug, file, title) {
	const snippetPath = join(ROOT, file);
	if (!existsSync(snippetPath) || !statSync(snippetPath).isFile()) {
		throw new Error(`${slug}: snippet ${file} does not exist`);
	}
	return `<CodeSnippet file="${file}" title="${title}" />`;
}

function renderCurl(auth, endpoint, method, pathParams, queryParams, requestSchema, resolveRef) {
	const examples = endpoint.examples ?? {};
	let url = endpoint.path;
	for (const p of pathParams) {
		const value = examples.path?.[p.name] ?? `$${shellVar(p.name)}`;
		url = url.replace(`{${p.name}}`, value);
	}

	const query = new URLSearchParams();
	for (const p of queryParams) {
		if (examples.query && p.name in examples.query) {
			query.set(p.name, String(examples.query[p.name]));
		} else if (p.required) {
			query.set(p.name, `$${shellVar(p.name)}`);
		}
	}
	if (examples.selector) {
		for (const [name, value] of Object.entries(examples.selector)) query.set(name, String(value));
	}
	// URLSearchParams percent-encodes `$`; keep shell variables readable.
	const queryString = query.toString().replaceAll("%24", "$");
	const fullUrl = `${config.baseUrl}${url}${queryString ? `?${queryString}` : ""}`;

	const parts = [];
	const upper = method.toUpperCase();
	parts.push(upper === "GET" ? `curl "${fullUrl}"` : `curl -X ${upper} "${fullUrl}"`);
	for (const header of auth.curlHeaders) parts.push(`  -H "${header}"`);

	if (requestSchema) {
		const body =
			examples.body ?? exampleFor(resolveRef(requestSchema), resolveRef, { name: "", optional: false }) ?? {};
		parts.push(`  -H "Content-Type: application/json"`);
		const json = JSON.stringify(body, null, 2);
		if (json === "{}") {
			parts.push(`  -d '{}'`);
		} else {
			parts.push(`  -d '${json.split("\n").join("\n  ")}'`);
		}
	}
	return parts.join(" \\\n");
}

function paramTable(params, describe, resolveRef) {
	const rows = params.map((p) => {
		const schema = p.schema ? resolveRef(p.schema) : {};
		return [
			code(p.name),
			code(typeLabel(schema, resolveRef)),
			p.required ? "Yes" : "No",
			cell(joinSentences(describe(p), enumSentence(schema))),
		];
	});
	return table(["Name", "Type", "Required", "Description"], rows);
}

function fieldTable(fields, { showRequired }) {
	const header = showRequired ? ["Field", "Type", "Required", "Description"] : ["Field", "Type", "Description"];
	const rows = fields.map((f) =>
		showRequired
			? [code(f.name), code(f.type), f.required ? "Yes" : "No", cell(f.description)]
			: [code(f.name), code(f.type), cell(f.description)],
	);
	return table(header, rows);
}

function table(header, rows) {
	const out = [];
	out.push(`| ${header.join(" | ")} |`);
	out.push(`| ${header.map(() => "---").join(" | ")} |`);
	for (const row of rows) out.push(`| ${row.join(" | ")} |`);
	return out.join("\n");
}

// ---------------------------------------------------------------------------
// Schema helpers
// ---------------------------------------------------------------------------

function isHttpMethod(key) {
	return ["get", "put", "post", "delete", "options", "head", "patch", "trace"].includes(key);
}

function deref(doc, node) {
	let current = node;
	const seen = new Set();
	while (current && typeof current === "object" && typeof current.$ref === "string") {
		if (seen.has(current.$ref)) throw new Error(`circular $ref ${current.$ref}`);
		seen.add(current.$ref);
		const target = current.$ref.replace(/^#\//, "").split("/").reduce((acc, key) => acc?.[key], doc);
		if (!target) throw new Error(`unresolved $ref ${current.$ref}`);
		const { $ref, ...rest } = current;
		current = { ...target, ...rest };
	}
	return current;
}

/** Splits a schema into (non-null types[], nullable) handling OAS 3.0 `nullable` and 3.1 type arrays. */
function schemaTypes(schema) {
	const raw = Array.isArray(schema.type) ? schema.type : schema.type ? [schema.type] : [];
	const nullable = schema.nullable === true || raw.includes("null");
	return { types: raw.filter((t) => t !== "null"), nullable };
}

function typeLabel(schema, resolveRef, depth = 0) {
	if (!schema || Object.keys(schema).length === 0) return "any";
	const variants = schema.oneOf ?? schema.anyOf;
	if (variants) {
		return variants.map((v) => typeLabel(resolveRef(v), resolveRef, depth + 1)).join(" | ");
	}
	if (schema.allOf) {
		return typeLabel(mergeAllOf(schema, resolveRef), resolveRef, depth);
	}
	const { types, nullable } = schemaTypes(schema);
	let base;
	if (types.length === 0) {
		base = schema.properties ? "object" : schema.enum ? "string" : "any";
	} else if (types.length > 1) {
		base = types.join(" | ");
	} else {
		const type = types[0];
		if (type === "array") {
			const items = schema.items ? resolveRef(schema.items) : {};
			const inner = typeLabel(items, resolveRef, depth + 1);
			base = inner.includes(" | ") ? `(${inner})[]` : `${inner}[]`;
		} else if (type === "object" && depth > 0 && schema.properties) {
			base = "object";
		} else {
			base = type;
		}
	}
	if (schema.enum && depth > 0) {
		base = schema.enum.map((v) => JSON.stringify(v)).join(" | ");
	}
	return nullable ? `${base} | null` : base;
}

function enumSentence(schema) {
	if (!schema?.enum) return "";
	return `One of ${schema.enum.map((v) => `\`${JSON.stringify(v)}\``).join(", ")}.`;
}

function mergeAllOf(schema, resolveRef) {
	const merged = { type: "object", properties: {}, required: [] };
	for (const part of schema.allOf) {
		const s = resolveRef(part);
		Object.assign(merged.properties, s.properties ?? {});
		merged.required.push(...(s.required ?? []));
	}
	return merged;
}

/**
 * Flattens an object schema into rows with dotted paths. Arrays of objects
 * are expanded as `name[].field`. Unions are described inline, not expanded.
 */
function flattenSchema(
	schema,
	resolveRef,
	describe = (_path, prop) => prop.description ?? "",
	prefix = "",
	depth = 0,
	out = [],
) {
	if (!schema || depth > 5) return out;
	if (schema.allOf) schema = mergeAllOf(schema, resolveRef);
	const { types } = schemaTypes(schema);
	const isObject = types.includes("object") || (types.length === 0 && schema.properties);
	if (!isObject || !schema.properties) return out;
	const required = new Set(schema.required ?? []);
	for (const [name, rawProp] of Object.entries(schema.properties)) {
		const prop = resolveRef(rawProp);
		const path = prefix ? `${prefix}.${name}` : name;
		// Enum members already appear in the type column at depth 1, so skip enumSentence here.
		out.push({
			name: path,
			type: typeLabel(prop, resolveRef, 1),
			required: required.has(name),
			description: joinSentences(describe(path, prop), deprecatedSentence(prop)),
		});
		const { types: propTypes } = schemaTypes(prop);
		if (propTypes.includes("object") || (propTypes.length === 0 && prop.properties)) {
			flattenSchema(prop, resolveRef, describe, path, depth + 1, out);
		} else if (propTypes.includes("array") && prop.items) {
			const items = resolveRef(prop.items);
			flattenSchema(items, resolveRef, describe, `${path}[]`, depth + 1, out);
		}
	}
	return out;
}

function deprecatedSentence(schema) {
	return schema.deprecated ? "Deprecated." : "";
}

/**
 * Builds a placeholder example value. Required fields are always included;
 * optional fields only when `optional` is set (used for responses).
 */
function exampleFor(schema, resolveRef, { name, optional }, depth = 0) {
	if (!schema || depth > 6) return undefined;
	if (schema.example !== undefined) return schema.example;
	if (schema.default !== undefined) return schema.default;
	if (schema.allOf) schema = mergeAllOf(schema, resolveRef);
	const variants = schema.oneOf ?? schema.anyOf;
	if (variants) return exampleFor(resolveRef(variants[0]), resolveRef, { name, optional }, depth + 1);
	if (schema.enum) return schema.enum[0];
	const { types, nullable } = schemaTypes(schema);
	const type = types[0];
	if (type === undefined) {
		if (schema.properties) return objectExample(schema, resolveRef, optional, depth);
		return nullable ? null : name ? `<${name}>` : {};
	}
	switch (type) {
		case "object":
			if (!schema.properties) return {};
			return objectExample(schema, resolveRef, optional, depth);
		case "array": {
			const items = schema.items ? resolveRef(schema.items) : {};
			const item = exampleFor(items, resolveRef, { name: singular(name), optional }, depth + 1);
			return item === undefined ? [] : [item];
		}
		case "string":
			return stringExample(name, schema);
		case "integer":
		case "number":
			return name.endsWith("_ts") ? 1700000000000 : 0;
		case "boolean":
			return false;
		default:
			return nullable ? null : undefined;
	}
}

function objectExample(schema, resolveRef, optional, depth) {
	const required = new Set(schema.required ?? []);
	const out = {};
	for (const [key, rawProp] of Object.entries(schema.properties)) {
		if (!optional && !required.has(key)) continue;
		const prop = resolveRef(rawProp);
		if (prop.deprecated) continue;
		// Nullable fields are shown as null so placeholders do not imply a state
		// (destroyed, errored, sleeping) that contradicts the rest of the example.
		const value = schemaTypes(prop).nullable
			? null
			: exampleFor(prop, resolveRef, { name: key, optional }, depth + 1);
		if (value !== undefined) out[key] = value;
	}
	return out;
}

function stringExample(name, schema) {
	if (schema.format === "uuid" || name === "actor_id" || name === "namespace_id" || name.endsWith("_id")) {
		return "00000000-0000-0000-0000-000000000000";
	}
	if (name === "namespace") return "my-namespace";
	if (name === "name") return "my-actor";
	if (name === "key") return "my-key";
	if (name === "datacenter" || name === "region") return "us-east";
	if (name === "runner_name_selector" || name === "pool" || name === "runner") return "default";
	if (name === "token") return "sk_...";
	if (name === "cursor") return "...";
	return name ? `<${name}>` : "...";
}

function singular(name) {
	return name.endsWith("s") ? name.slice(0, -1) : name;
}

// ---------------------------------------------------------------------------
// Text helpers
// ---------------------------------------------------------------------------

function shellVar(name) {
	return name.replace(/[^a-zA-Z0-9]/g, "_").toUpperCase();
}

function joinSentences(...parts) {
	return parts.filter((p) => p && p.trim()).join(" ");
}

/** Escapes characters MDX would otherwise treat as JSX or expressions. */
function mdxText(text) {
	if (!text) return "";
	// Leave inline code untouched; escape everything else.
	return text
		.split(/(`[^`]*`)/)
		.map((part, i) => (i % 2 === 1 ? part : part.replace(/[{}<>]/g, (c) => `\\${c}`)))
		.join("");
}

/** Table cell: single line, escaped pipes, MDX-safe. */
function cell(text) {
	return mdxText((text ?? "").replace(/\s*\n\s*/g, " ").trim()).replaceAll("|", "\\|");
}

/** Inline code inside a table cell. Pipes must be escaped even inside backticks. */
function code(text) {
	return `\`${text.replaceAll("|", "\\|")}\``;
}

function readJson(path) {
	return JSON.parse(readFileSync(path, "utf8"));
}
