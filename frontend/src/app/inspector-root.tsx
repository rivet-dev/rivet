import { CatchBoundary, useSearch } from "@tanstack/react-router";
import { useEffect, useRef } from "react";
import { askForLocalNetworkAccess } from "@/lib/permissions";
import { Actors } from "./actors";
import { BuildPrefiller } from "./build-prefiller";
import { Connect } from "./connect";
import { useInspectorContext } from "./inspector-context";
import { Logo } from "./logo";
import { RouteLayout } from "./route-layout";

export function InspectorRoot() {
	const { isInspectorAvailable, connect } = useInspectorContext();
	const search = useSearch({ from: "/_context" });

	const formRef = useRef<HTMLFormElement>(null);

	useEffect(() => {
		formRef.current?.requestSubmit();
	}, []);

	if (isInspectorAvailable) {
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
						const hasLocalNetworkAccess =
							await askForLocalNetworkAccess();

						if (!hasLocalNetworkAccess) {
							form.setError("url", {
								message:
									"Local network access is required to connect to local RivetKit. Please enable local network access in your browser settings and try again.",
							});
							return;
						}

						try {
							const response = await fetch(values.url, {
								method: "OPTIONS",
							});
							if (!response.ok) {
								throw new Error("CORS preflight failed");
							}

							await connect({ url: values.url });
						} catch {
							form.setError("url", {
								message: "localhost.cors.error",
							});
						}
					}}
				/>
			</div>
		</div>
	);
}
