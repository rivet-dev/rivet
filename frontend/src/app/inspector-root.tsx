import {
	CatchBoundary,
	useLocation,
	useNavigate,
	useRouteContext,
	useSearch,
} from "@tanstack/react-router";
import { useRef } from "react";
import { match } from "ts-pattern";
import { Actors } from "./actors";
import { BuildPrefiller } from "./build-prefiller";
import { Connect } from "./connect";
import { Logo } from "./logo";
import { RouteLayout } from "./route-layout";

export function InspectorRoot() {
	const connectedInPreflight = useRouteContext({
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
	const connectedInForm = useLocation({
		select: (loc) =>
			"connectedInForm" in loc.state
				? (loc.state.connectedInForm ?? false)
				: false,
	});

	const alreadyConnected = connectedInPreflight || connectedInForm;

	const navigate = useNavigate();
	const search = useSearch({ from: "/_context" });

	const formRef = useRef<HTMLFormElement>(null);

	if (alreadyConnected) {
		return (
			<RouteLayout>
				<Actors actorId={search.actorId} />
				<CatchBoundary
					getResetKey={() => search.n?.join(",") ?? "no-build-name"}
					errorComponent={() => null}
				>
					{!search.n ? <BuildPrefiller /> : null}
				</CatchBoundary>
			</RouteLayout>
		);
	}

	return (
		<div className="flex min-h-screen flex-col items-center justify-center bg-background py-4">
			<div className="flex flex-col items-center gap-6 w-full">
				<Logo className="h-10 mb-4" />
				<Connect
					formRef={formRef}
					onSubmit={async (values, form) => {
						try {
							const response = await fetch(values.url, {
								method: "OPTIONS",
							});
							if (!response.ok) {
								throw new Error("CORS preflight failed");
							}
							await navigate({
								to: "/",
								search: (old) => {
									return {
										...old,
										u: values.url,
									};
								},
								state: (old) => ({
									...old,
									connectedInForm: true,
								}),
							});
						} catch {
							form.setError("url", {
								message:
									"Failed to connect. Please check your URL, and CORS settings.",
							});
						}
					}}
				/>
			</div>
		</div>
	);
}
