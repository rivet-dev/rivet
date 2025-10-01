import { Suspense } from "react";
import { Skeleton } from "../ui/skeleton";
import { ActorBuild } from "./actor-build";
import type { ActorId } from "./queries";

export interface ActorRuntimeProps {
	actorId: ActorId;
}

export function ActorRuntime({ actorId }: ActorRuntimeProps) {
	return (
		<Suspense fallback={<Skeleton className="w-full h-32 col-span-2" />}>
			<ActorBuild actorId={actorId} />
		</Suspense>
	);
}
