import fs from 'fs';
import YAML from 'yaml';
import { flattenOpenAPISpec } from './flattenSpec.mjs';
import { generateApiPages } from './generateApiPages.mjs';
import { generateRivetSchema } from './generateRivetSchema.mjs';
import { generateRivetSchemaPage } from './generateRivetSchemaPage.mjs';

let BACKEND_PATH = '../rivet-ee';

export async function main() {
  // Read spec
  let specPath = `${BACKEND_PATH}/sdks/full/openapi/openapi.yml`;
  const fileContents = fs.readFileSync(specPath, 'utf8');
  const fullSpec = YAML.parse(fileContents, { maxAliasCount: -1 });

  // Flatten spec
  const flatSpec = await flattenOpenAPISpec(fullSpec);

  // Generate API pages
  await generateApiPages(flatSpec);

  // Generate rivet.yaml schema
  await generateRivetSchema(fullSpec);

  // Generate rivet.yaml schema docs
  await generateRivetSchemaPage(flatSpec);
}

main();
