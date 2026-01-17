import { faRailway, Icon } from "@rivet-gg/icons";
import { Link } from "@tanstack/react-router";
import { Button } from "../components/ui/button";

export function OneClickDeployRailwayButton() {
	return (
		<Button
			size="lg"
			variant="outline"
			className="min-w-48 h-auto min-h-28 text-xl"
			startIcon={<Icon icon={faRailway} />}
			asChild
		>
			<Link to="." search={{ modal: "connect-q-railway" }}>
				Railway
			</Link>
		</Button>
	);
}
