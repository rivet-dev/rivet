import fs from 'fs';

// Beware when changing this path, as it is referenced
// within the schema store
// 
// If it requires a change for some critical reason
// refer to https://github.com/SchemaStore/schemastore/pull/3485
const SCHEMA_LOCATION = "public/rivet.schema.json";

export async function generateRivetSchema(spec) {
  // Deep clone since we'll be modifying this directly
  spec = JSON.parse(JSON.stringify(spec))

  const openapiRefs = spec.components.schemas;

  // Version upload endpoint
  const endpoint = spec.paths["/cloud/games/{game_id}/versions"]["post"];
  const bodySchema = readPastRefs(endpoint.requestBody.content['application/json'].schema, openapiRefs);

  const configSchema = readPastRefs(bodySchema.properties.config, openapiRefs);
  const refsBeingUsed = [];
  transformToV7Schema(configSchema, refsBeingUsed);

  const definitions = {}
  for (let i = 0; i < refsBeingUsed.length; ++i) {
    const name = refsBeingUsed[i];
    const definition = openapiRefs[name];
    // refsBeingUsed grows while this is happening
    transformToV7Schema(definition, refsBeingUsed);
    definitions[name] = definition;
  }

  const standardSchema = {
    $schema: "http://json-schema.org/draft-07/schema",
    title: "rivet.yaml Config",
    type: configSchema.type,
    properties: configSchema.properties,
    definitions: definitions
  }

  fs.writeFileSync(SCHEMA_LOCATION, JSON.stringify(standardSchema, null, 2));
}

// Rough converter OpenAPI schema into JSON schema (v7) draft
function transformToV7Schema(openapiSchema, refsBeingUsed) {
  // Iterate properties
  if (openapiSchema?.properties) {
    for (let property in openapiSchema.properties) {
    transformToV7Schema(openapiSchema.properties[property], refsBeingUsed);
    }
  }

  // Iterate arrays
  if (openapiSchema?.items) {
    transformToV7Schema(openapiSchema.items, refsBeingUsed);
  }

  // Iterate additional properties
  if (openapiSchema?.additionalProperties) {
    transformToV7Schema(openapiSchema.additionalProperties, refsBeingUsed);
  }


  // Resolve refs
  if (openapiSchema?.$ref) {
    const ref = openapiSchema.$ref;
    const name = ref.replace(/^\#\/components\/schemas\//, '');

    if (!refsBeingUsed.includes(name)) refsBeingUsed.push(name)

    openapiSchema.$ref = '#/definitions/' + name;
  }
}

function readPastRefs(schema, openapiRefs) {
  while (schema["$ref"]) {
    const refPath = schema["$ref"].replace('#/components/schemas/', '');

    schema = openapiRefs[refPath];
  }

  return schema;
}

