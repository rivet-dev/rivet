export type NextRouteContext = {
	readonly params?:
		| Promise<{ readonly all?: readonly string[] }>
		| { readonly all?: readonly string[] };
};

export type FetchApp = {
	fetch(request: Request, env?: unknown): Response | Promise<Response>;
};

type NextRouteHandler = (request: Request, context: NextRouteContext) => Promise<Response>;

export function toFlueNextHandler(app: FetchApp): Record<string, NextRouteHandler> {
	const fetchWrapper: NextRouteHandler = async (request, context) => {
		const params = context.params ? await context.params : {};
		const all = params.all ?? [];
		const targetUrl = new URL(request.url);
		targetUrl.pathname = `/${all.map((segment) => encodeURIComponent(segment)).join('/')}`;
		return app.fetch(new Request(targetUrl, request));
	};
	return {
		GET: fetchWrapper,
		POST: fetchWrapper,
		PUT: fetchWrapper,
		DELETE: fetchWrapper,
		PATCH: fetchWrapper,
		HEAD: fetchWrapper,
		OPTIONS: fetchWrapper,
	};
}
