/// Reads a schema and flattens $refs.
export async function flattenOpenAPISpec(spec) {
  // Deep clone since we'll be modifying this directly
  spec = JSON.parse(JSON.stringify(spec))

  let schemas = spec.components.schemas;

  for (let pathKey in spec.paths) {
    let path = spec.paths[pathKey];

    for (let methodKey in path) {
      let method = path[methodKey];

      if (method.requestBody?.content) {
        let requestBodySchema = method.requestBody.content['application/json'].schema;
        method.requestBody.content['application/json'].schema = flattenRefs(requestBodySchema, schemas);
      }

      for (let response in method.responses) {
        if (!method.responses[response].content) continue;
        let responseSchema = method.responses[response].content['application/json'].schema;

        method.responses[response].content['application/json'].schema = flattenRefs(responseSchema, schemas);
      }
    }
  }

  return spec;
}

/// Flattens $ref in to the root object. We use this for exposing the full schema in the docs.
function flattenRefs(schema, schemas) {
  // Exclude if deprecated
  if (schema?.description?.indexOf('Deprecated') >= 0) return null;

  // Iterate properties
  if (schema?.properties) {
    for (let property in schema.properties) {
      schema.properties[property] = flattenRefs(schema.properties[property], schemas);
    }
  }

  // Iterate parameters
  if (schema?.parameters) {
    for (let parameter in schema.parameters) {
      schema.parameters[parameter].schema = flattenRefs(schema.parameters[parameter].schema, schemas);
    }
  }

  // Iterate arrays
  if (schema?.items) {
    schema.items = flattenRefs(schema.items, schemas);
  }

  // Iterate additional properties
  if (schema?.additionalProperties) {
    schema.additionalProperties = flattenRefs(schema.additionalProperties, schemas);
  }

  // Resolve refs
  if (schema?.$ref) {
    let ref = schema.$ref;
    let refPath = ref.replace('#/components/schemas/', '');
    let refSchema = JSON.parse(JSON.stringify(flattenRefs(schemas[refPath], schemas)));
    if (schema.description) refSchema.description = schema.description;
    return refSchema;
  }

  // No ref
  return schema;
}
