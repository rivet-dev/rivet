import { Button } from "@rivet-gg/components";
import { Icon, faChevronDown } from "@rivet-gg/icons";
import { AnimatePresence, motion } from "framer-motion";
import { useState } from "react";
import { useActorWorkerStatus } from "../worker/actor-worker-context";
import { ActorWorkerStatus } from "../worker/actor-worker-status";
import { ActorConsoleInput } from "./actor-console-input";
import { ActorConsoleLogs } from "./actor-console-logs";

export function ActorConsole() {
	const [isOpen, setOpen] = useState(false);

	const status = useActorWorkerStatus();

	const isBlocked = status.type !== "ready";

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
				className="border-t border-b rounded-none w-full justify-between min-h-9 disabled:opacity-100 aria-disabled:opacity-100"
				size="sm"
				endIcon={isBlocked ? undefined : <Icon icon={faChevronDown} />}
			>
				<span>
					Console
					<ActorWorkerStatus status={status.type} />
				</span>
			</Button>
			<AnimatePresence>
				{isOpen && !isBlocked ? (
					<motion.div
						exit={{ opacity: 0 }}
						className="flex flex-col flex-1 max-h-full overflow-hidden"
					>
						<ActorConsoleLogs />
						<ActorConsoleInput />
					</motion.div>
				) : null}
			</AnimatePresence>
		</motion.div>
	);
}
