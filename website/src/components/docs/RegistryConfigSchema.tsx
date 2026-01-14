import { JsonSchemaPreview } from "@/components/JsonSchemaPreview";
import registryConfigSchema from "../../../../rivetkit-typescript/artifacts/registry-config.json";

export function RegistryConfigSchema() {
	return (
		<JsonSchemaPreview
			schema={registryConfigSchema}
			empty={<p className="text-muted-foreground">No properties</p>}
		/>
	);
}
