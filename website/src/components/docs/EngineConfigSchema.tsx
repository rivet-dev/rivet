import { JsonSchemaPreview } from "@/components/JsonSchemaPreview";
import engineConfigSchema from "../../../../engine/artifacts/config-schema.json";

export function EngineConfigSchema() {
	return (
		<JsonSchemaPreview
			schema={engineConfigSchema}
			empty={<p className="text-muted-foreground">No properties</p>}
		/>
	);
}
