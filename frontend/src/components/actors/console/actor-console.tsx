import { faChevronDown, Icon } from "@rivet-gg/icons";
import { useQuery } from "@tanstack/react-query";
import { AnimatePresence, motion } from "framer-motion";
import { useState } from "react";
import { Button, cn } from "@/components";
import { useActor } from "../actor-queries-context";
import { useManager } from "../manager-context";
import type { ActorId } from "../queries";
import { useActorWorkerStatus } from "../worker/actor-worker-context";
import { ActorWorkerStatus } from "../worker/actor-worker-status";
import { ActorConsoleInput } from "./actor-console-input";
import { ActorConsoleLogs } from "./actor-console-logs";

interface ActorConsoleProps {
	actorId: ActorId;
}

export function ActorConsole({ actorId }: ActorConsoleProps) {
	const [isOpen, setOpen] = useState(false);

	const status = useActorWorkerStatus();
	const managerQueries = useManager();
	const actorQueries = useActor();
	const { data: destroyedAt } = useQuery(
		managerQueries.actorDestroyedAtQueryOptions(actorId),
	);
	const { isSuccess, isError, isLoading } = useQuery(
		actorQueries.actorPingQueryOptions(actorId, {
			enabled: true,
			refetchInterval: false,
		}),
	);

	const isBlocked = status.type !== "ready" || !isSuccess || !!destroyedAt;

	const combinedStatus = isError
		? "error"
		: isLoading
			? "pending"
			: status.type;

	return (
		<motion.div
			animate={{
				height: isOpen && !isBlocked ? "50%" : "36px",
			}}
			className="overflow-hidden flex flex-col"
		>
			<Button
				variant="ghost"
				disabled={isBlocked}
				onClick={() => setOpen((old) => !old)}
				className={cn(
					!isOpen ? " border-b-0" : "border-b",
					"border-t border-border border-l-0 border-r-0 rounded-none w-full justify-between min-h-9 disabled:opacity-100 aria-disabled:opacity-100",
				)}
				size="sm"
				endIcon={isBlocked ? undefined : <Icon icon={faChevronDown} />}
			>
				<span>
					Console
					{isBlocked ? null : (
						<ActorWorkerStatus status={combinedStatus} />
					)}
				</span>
			</Button>
			<AnimatePresence>
				{isOpen && !isBlocked ? (
					<motion.div
						exit={{ opacity: 0 }}
						className="flex flex-col flex-1 max-h-full overflow-hidden"
					>
						<ActorConsoleLogs />
						<ActorConsoleInput actorId={actorId} />
					</motion.div>
				) : null}
			</AnimatePresence>
		</motion.div>
	);
}
