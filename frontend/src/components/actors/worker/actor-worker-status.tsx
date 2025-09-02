import { faExclamationTriangle, faSpinner, Icon } from "@rivet-gg/icons";
import { AnimatePresence, motion } from "framer-motion";
import type { ContainerStatus } from "./actor-worker-container";

interface ActorWorkerStatusProps {
	status: ContainerStatus["type"];
}

export function ActorWorkerStatus({ status }: ActorWorkerStatusProps) {
	return (
		<AnimatePresence>
			{status === "pending" ? (
				<motion.span
					className="text-muted-foreground"
					exit={{ opacity: 0 }}
				>
					<Icon icon={faSpinner} className="animate-spin ml-2 mr-1" />
					Connecting to Actor...
				</motion.span>
			) : null}
			{status === "error" ? (
				<motion.span
					className="text-red-400 bg-red-950/30 border-red-800/40  px-2.5 py-0.5 border rounded-full ml-2"
					exit={{ opacity: 0 }}
				>
					<Icon icon={faExclamationTriangle} className="mr-1" />
					Couldn't connect to Actor.
				</motion.span>
			) : null}
			{status === "unsupported" ? (
				<motion.span
					className="bg-yellow-500/10 border-yellow-800/40 text-yellow-200  px-2.5 py-0.5 border rounded-full ml-2"
					exit={{ opacity: 0 }}
				>
					<Icon icon={faExclamationTriangle} className="mr-1" />
					Console is not supported for this Actor.
				</motion.span>
			) : null}
		</AnimatePresence>
	);
}
