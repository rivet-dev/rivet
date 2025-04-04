import { faActors, Icon } from "@rivet-gg/icons";
import { ActorsResources } from "./get-started";

export function GettingStarted() {
	return (
		<div className="w-full h-full flex flex-col justify-center">
			<div className="flex flex-col justify-center my-8">
				<Icon icon={faActors} className="text-6xl mx-auto my-4" />
				<h3 className="text-center font-bold text-xl max-w-md mb-2 mx-auto">
					Deploy your first Actor
				</h3>
				<p className="text-center text-muted-foreground max-w-sm mx-auto">
					Use a quick start guide to start deploying Actors to your
					environment.
				</p>
			</div>
			<ActorsResources />
		</div>
	);
}
