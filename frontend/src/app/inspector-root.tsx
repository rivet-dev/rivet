import {
	CatchBoundary,
	useNavigate,
	useRouteContext,
	useSearch,
} from "@tanstack/react-router";
import { useEffect, useMemo, useRef, useState } from "react";
import { match } from "ts-pattern";
import { askForLocalNetworkAccess } from "@/lib/permissions";
import { Actors } from "./actors";
import { BuildPrefiller } from "./build-prefiller";
import { Connect } from "./connect";
import { InspectorCredentialsProvider } from "./credentials-context";
import { createClient } from "./data-providers/inspector-data-provider";
import { Logo } from "./logo";
import { RouteLayout } from "./route-layout";

export function InspectorRoot() {
	const alreadyConnected = useRouteContext({
		from: "/_context/",
		select: (ctx) =>
			match(ctx)
				.with({ __type: "inspector" }, (c) =>
					"connectedInPreflight" in c
						? c.connectedInPreflight
						: false,
				)
				.otherwise(() => null),
	});
	const navigate = useNavigate();
	const search = useSearch({ from: "/_context" });
	const [credentials, setCredentials] = useState<null | {
		url: string;
		token: string;
	}>(alreadyConnected ? { url: search.u!, token: search.t! } : null);

	const formRef = useRef<HTMLFormElement>(null);

	useEffect(() => {
		if (search.t) {
			formRef.current?.requestSubmit();
		}
	}, [search.t]);

	const ctxValue = useMemo(() => {
		return { credentials, setCredentials };
	}, [credentials]);

	if (credentials || alreadyConnected) {
		return (
			<InspectorCredentialsProvider value={ctxValue}>
				<RouteLayout>
					<Actors actorId={search.actorId} />
					<CatchBoundary
						getResetKey={() =>
							search.n?.join(",") ?? "no-build-name"
						}
						errorComponent={() => null}
					>
						{!search.n ? <BuildPrefiller /> : null}
					</CatchBoundary>
				</RouteLayout>
			</InspectorCredentialsProvider>
		);
	}

	return (
		<div className="flex min-h-screen flex-col items-center justify-center bg-background py-4">
			<div className="flex flex-col items-center gap-6 w-full">
				<Logo className="h-10 mb-4" />
				<Connect
					formRef={formRef}
					onSubmit={async (values, form) => {
						const hasLocalNetworkAccess =
							await askForLocalNetworkAccess();

						if (!hasLocalNetworkAccess) {
							form.setError("token", {
								message:
									"Local network access is required to connect to local RivetKit. Please enable local network access in your browser settings and try again.",
							});
							return;
						}

						try {
							const client = createClient({
								url: values.username,
								token: values.token,
							});
							const resp = await client.ping.$get();
							if (!resp.ok) {
								throw resp;
							}
							await navigate({
								to: "/",
								search: (old) => {
									return {
										...old,
										u: values.username,
										t: values.token,
									};
								},
							});
							setCredentials({
								url: values.username,
								token: values.token,
							});
						} catch {
							form.setError("token", {
								message:
									"Failed to connect. Please check your URL and token.",
							});
						}
					}}
				/>
			</div>
		</div>
	);
}
