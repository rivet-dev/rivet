import { Badge } from "@rivet-gg/components";
import { motion } from "framer-motion";
import { useEffect, useRef } from "react";

interface ActorStateChangeIndicatorProps {
	state: string | undefined;
}

export function ActorStateChangeIndicator({
	state,
}: ActorStateChangeIndicatorProps) {
	const isMounted = useRef(false);
	const oldState = useRef("");

	useEffect(() => {
		isMounted.current = true;
	}, []);

	useEffect(() => {
		oldState.current = state || "";
	}, [state]);

	const hasChanged = state !== oldState.current;
	const shouldUpdate = hasChanged && isMounted.current;

	return (
		<Badge asChild>
			<motion.div
				key={state}
				initial={{ opacity: shouldUpdate ? 1 : 0 }}
				animate={{ opacity: 0, transition: { delay: 1 } }}
			>
				State changed
			</motion.div>
		</Badge>
	);
}
