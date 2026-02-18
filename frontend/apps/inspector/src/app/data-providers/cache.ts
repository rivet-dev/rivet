import { createGlobalContext as createGlobalInspectorContext } from "@/app/data-providers/inspector-data-provider";

export type InspectorContext = ReturnType<typeof createGlobalInspectorContext>;

const inspectorContextCache = new Map<string, InspectorContext>();

export function getOrCreateInspectorContext(opts: {
	url?: string;
	token?: string;
}): InspectorContext {
	const key = `${opts.url ?? ""}:${opts.token ?? ""}`;
	const cached = inspectorContextCache.get(key);
	if (cached) {
		return cached;
	}
	const context = createGlobalInspectorContext(opts);
	inspectorContextCache.set(key, context);
	return context;
}
