import fs from 'fs';
import path from 'path';

import { convertJsonSchemaToSchema, parametersToHTML } from './convertJson.mjs';

function apiPath(productName) {
  return `src/docs/general/${productName}/api`;
}

function camelToKebab(input) {
  return input.replace(/(.)([A-Z])/g, '$1-$2').toLowerCase();
}

let PRODUCTS = {
  'dynamic-servers': {
    roots: [
      '/games/{game_id}/environments/{environment_id}/servers',
      '/games/{game_id}/environments/{environment_id}/datacenters',
      '/games/{game_id}/environments/{environment_id}/builds'
    ],
    operationIdPrefix: 'servers',
    importantEndpoints: []
  },
  'game-tokens': {
    roots: ['/games/{game_id}/environments/{environment_id}/tokens'],
    operationIdPrefix: 'games_environments_tokens',
    importantEndpoints: []
  },
  cloud: {
    roots: ['/cloud'],
    importantEndpoints: [
      'POST /cloud/games/{game_id}/versions',
      'PUT /cloud/games/{game_id}/namespaces/{namespace_id}/version',
      'POST /cloud/games/{game_id}/namespaces'
    ]
  }
};

export async function generateApiPages(spec) {
  let apiPages = {};

  const apiBaseUrl = spec.servers[0].url;

  for (let product in PRODUCTS) {
    fs.rmSync(apiPath(product), { recursive: true, force: true });
    fs.mkdirSync(apiPath(product), { recursive: true });
  }

  for (let pathName in spec.paths) {
    for (let method in spec.paths[pathName]) {
      let specPath = spec.paths[pathName][method];

      console.log('Registering', method, pathName);

      let fullUrl = apiBaseUrl + pathName;

      // Find product config
      let productName;
      for (let product in PRODUCTS) {
        for (let root of PRODUCTS[product].roots) {
          if (pathName.startsWith(root)) {
            productName = product;
            break;
          }
        }
      }
      if (!productName) continue;

      let productConfig = PRODUCTS[productName];

      let indexableName = `${method.toUpperCase()} ${pathName}`;
      let importantIndex = productConfig.importantEndpoints.indexOf(indexableName);
      let isImportant = importantIndex != -1;

      let experimentalIndex = productConfig.experimentalEndpoints?.indexOf(indexableName) ?? -1;
      let isExperimental = experimentalIndex != -1 || productConfig.isExperimental;

      // Remove product prefix from operation ID
      let operationIdStripped = specPath.operationId
        .replace(`ee_`, '')
        .replace(`${productConfig.operationIdPrefix ?? productName}_`, '');

      // Generate title
      let title = operationIdStripped.replace(/_/g, '.');
      if (isImportant) title = '⭐️ ' + title;

      // Get description with a default fallback
      let description = '*Coming Soon!*';
      if (specPath.description) description = specPath.description;

      let hasRequestBody = specPath.requestBody?.content['application/json']?.schema;

      let file = `
# ${title}

${isExperimental ? '<ExperimentalFeature />' : ''}

## Description

${description}

`;

      // Code examples
      let curlCommand;
      if (hasRequestBody) {
        curlCommand = `# Write the request body to body.json before running\ncurl -X ${method.toUpperCase()} -d '@body.json' '${fullUrl}'`;
      } else {
        curlCommand = `curl -X ${method.toUpperCase()} '${fullUrl}'`;
      }
      file += `
## Code Examples

<CodeGroup title='Request' tag='${method.toUpperCase()}' label='${fullUrl}'>

\`\`\`bash {{ "title": "cURL" }}
${curlCommand}
\`\`\`

\`\`\`ts
// Create Rivet client
import { RivetClient } from '@rivet-gg/api';
const RIVET = new RivetClient({ token: '[YOUR TOKEN HERE]' });

// Make request
await RIVET.${specPath.operationId.replace(/_/g, '.')}({
  // Add your request body here
});
\`\`\`

</CodeGroup>

`;

      // Request parameters
      if (specPath.parameters) {
        // Don't include the schema because it's not useful
        file += `## Request Parameters\n`;
        file += parametersToHTML(specPath.parameters);
        file += '\n';
      }

      // Request body
      if (hasRequestBody) {
        ('');
        file += `## Request Body\n`;
        let reqSchema = specPath.requestBody?.content['application/json'].schema;
        if (reqSchema && Object.keys(reqSchema.properties).length > 0) {
          file += `<SchemaPreview schema={${JSON.stringify(convertJsonSchemaToSchema(reqSchema))}}/>`;
          file += '\n';
        } else {
          file += `_Empty request body._\n`;
        }
      }

      // Response body
      file += `## Response Body\n`;
      let resSchema = specPath.responses['200']?.content['application/json']?.schema;
      if (resSchema && Object.keys(resSchema.properties).length > 0) {
        file += `<SchemaPreview schema={${JSON.stringify(convertJsonSchemaToSchema(resSchema))}}/>`;
        file += '\n';
      } else {
        file += `_Empty response body._\n`;
      }

      let fileName = camelToKebab(operationIdStripped.replace(/\_/g, '/'));
      let filePath = `${apiPath(productName)}/${fileName}`;
      fs.mkdirSync(path.dirname(filePath), { recursive: true });

      // Sort by grouping similar endpoints together and move important endpoints first
      let sortingKey = `${isImportant ? '0' : `999 ${importantIndex}`} ${pathName} ${method}`;

      fs.writeFileSync(`${filePath}.mdx`, file);

      // Write config
      apiPages[productName] = apiPages[productName] || { pages: [] };
      apiPages[productName].pages.push({
        href: `/docs/general/${productName}/api/${fileName}`,
        sortingKey
      });
    }
  }

  // Sort pages
  for (let productName in apiPages) {
    apiPages[productName].pages.sort((a, b) => {
      if (a.sortingKey < b.sortingKey) return -1;
      else if (a.sortingKey > b.sortingKey) return 1;
      else return 0;
    });
  }

  fs.writeFileSync('src/generated/apiPages.json', JSON.stringify(apiPages, null, 2));
}
