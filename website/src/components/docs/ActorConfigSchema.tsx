import { JsonSchemaPreview } from "@/components/JsonSchemaPreview";
import actorConfigSchema from "../../../../rivetkit-typescript/artifacts/actor-config.json";

export function ActorConfigSchema() {
	return (
		<JsonSchemaPreview
			schema={actorConfigSchema}
			empty={<p className="text-ink-soft">No properties</p>}
		/>
	);
}
