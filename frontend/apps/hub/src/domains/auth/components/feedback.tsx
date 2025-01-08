import { Link as LinkCmp } from "@rivet-gg/components";
import { Link } from "@tanstack/react-router";

export function Feedback() {
	return (
		<LinkCmp asChild>
			<Link to="." search={{ modal: "feedback" }}>
				Missing something? Spot a bug?
			</Link>
		</LinkCmp>
	);
}
