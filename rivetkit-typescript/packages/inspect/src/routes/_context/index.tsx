import { faAngleDown, Icon } from "@rivet-gg/icons";
import {
	CatchBoundary,
	createFileRoute,
	useSearch,
} from "@tanstack/react-router";
import { useState } from "react";
import { Actors } from "@/app/actors";
import { BuildPrefiller } from "@/app/build-prefiller";
import { InspectorCredentialsProvider } from "@/app/credentials-context";
import { ConnectionStatus } from "@/app/layout";
import { RouteLayout } from "@/app/route-layout";
import rivetLogo from "../../assets/rivet.svg";

export const Route = createFileRoute("/_context/")({
	component: RouteComponent,
});

function RouteComponent() {
	const search = useSearch({ from: "/_context" });
	const [credentials, setCredentials] = useState({
		url: "http://127.0.0.1:6420",
		token: "lNtOy9TZxmt2yGukMJREcJcEp78WDzuj",
	});

	return (
		<InspectorCredentialsProvider value={{ credentials, setCredentials }}>
			<div className="flex items-center justify-between p-2 border-b">
				<div className="flex items-center gap-1">
					<img src={rivetLogo} className="h-6 w-6" />
					<span className="font-semibold">Rivet Inspector</span>
				</div>
				<div className="flex items-center gap-4">
					<ConnectionStatus className="text-sm min-w-32 text-center items-center justify-center flex py-0.5 px-1" />
					<button
						type="button"
						className="size-4 flex items-center justify-center"
						onClick={() => setIsOpen((prev) => !prev)}
					>
						<Icon icon={faAngleDown} />
					</button>
				</div>
			</div>
			<div className="flex-grow">
				<RouteLayout defaultCollapsed={true}>
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
			</div>
		</InspectorCredentialsProvider>
	);
}
