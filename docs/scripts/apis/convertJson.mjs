/// Generates Markdown docs for a given JSON schema.
export function jsonToMarkdown(schema, heading = 3) {
  let h = '#'.repeat(heading);

  let markdownContent = '';

  function documentProperty(value, required, path, skip = false) {
    if (!skip) {
      // Document property

      if (!value) {
        // markdownContent += `## \`${path}\`\n\n**Type:** null\n\n`;
        return;
      }

      let type = value.type || 'object';
      if (type == 'array') {
        type = `array\<${value.items?.type || 'object'}\>`;
      } else if (type == 'object' && value.additionalProperties) {
        type = `map\<string, ${value.additionalProperties?.type || 'object'}\>`;
      }

      markdownContent += `${h} \`${path}\`\n\n`;
      markdownContent += `\`${type}\`${required ? ' (required)' : ''}\n\n`;
      if (value.description) {
        markdownContent += `${value.description}\n\n`;
      }
    }

    // Recurse
    if (value.type == 'object' && value.properties) {
      let entries = Object.entries(value.properties);
      entries.sort((a, b) => a[0].localeCompare(b[0]));

      for (const [key, childValue] of entries) {
        let entryPath;
        if (path == '') entryPath = key;
        else entryPath = `${path}.${key}`;

        let required = value.required?.includes(key) ?? false;

        documentProperty(childValue, required, entryPath);
      }
    } else if (value.type == 'object' && value.additionalProperties) {
      documentProperty(value.additionalProperties, false, `${path}.<${getKeyName(path)}>`, true);
    } else if (value.type == 'array' && value.items) {
      documentProperty(value.items, false, `${path}[*]`, true);
    }
  }

  documentProperty(schema, true, '', true);

  return markdownContent;
}

function getKeyName(path) {
  let pathComponents = path.split('.');
  let lastComponent = pathComponents[pathComponents.length - 1];

  // Hardcoded
  switch (lastComponent) {
    case 'env':
      return 'key';
  }

  // YOLO it
  return stripSuffix(lastComponent, 's');
}

function stripSuffix(str, suffix) {
  return str.endsWith(suffix) ? str.slice(0, -suffix.length) : str;
}

export function jsonToHTML(schema) {
  function documentProperty(value, required, path, skip = false) {
    let propertyHTML = '';

    if (!skip) {
      // Skip deprecated
      if (!value) return '';

      let type = value.type || 'object';
      if (type == 'array') {
        type = `array&lt;${value.items?.type || 'object'}&gt;`;
      } else if (type == 'object' && value.additionalProperties) {
        type = `map&lt;string, ${value.additionalProperties?.type || 'object'}&gt;`;
      }
      let typeHTML = `<code className="m-0 ml-4 pb-0 pt-0.5 text-cream-500">${
        required ? `<span className="font-normal text-cream-600">required</span> ` : ``
      }${type}</code>`;

      propertyHTML += `<div className="property ml-4">\n`;
      propertyHTML += `<div className="flex flex-row items-center my-2">\n<h2 className="text-lg font-bold text-cream-50 m-0">${path.replace(
        '_',
        '\\_'
      )}</h2>${typeHTML}\n</div>\n`;

      if (value.description) {
        propertyHTML += `<div className="description pl-2">\n${value.description}\n</div>\n`;
      }
    }

    // Recurse
    if (value.type == 'object' && value.properties) {
      propertyHTML += `<div className="properties ${skip ? '' : 'ml-2 pl-2 border-l-2 border-cream-700'}">\n`;
      let entries = Object.entries(value.properties);
      entries.sort((a, b) => a[0].localeCompare(b[0]));

      for (const [key, childValue] of entries) {
        let required = value.required?.includes(key) ?? false;
        propertyHTML += documentProperty(childValue, required, key);
      }

      propertyHTML += `</div>\n`;
    } else if (value.type == 'object' && value.additionalProperties) {
      propertyHTML += `<div className="additional-properties ml-2 pl-2">\n`;
      propertyHTML += documentProperty(
        value.additionalProperties,
        false,
        `${path}.&lt;${getKeyName(path)}&gt;`
      );
      propertyHTML += `\n</div>\n`;
    } else if (value.type == 'array' && value.items) {
      propertyHTML += `<div className="array-items ml-2 pl-2">\n`;
      propertyHTML += documentProperty(value.items, false, `${path}[*]`);
      propertyHTML += `</div>\n`;
    }

    if (!skip) {
      propertyHTML += `</div>\n`;
    }

    return propertyHTML;
  }

  return documentProperty(schema, true, '', true);
}

export function parametersToHTML(parameters) {
  let parameterHTML = '';

  for (let parameter of parameters) {
    parameterHTML += `<div className="property ml-4">\n`;

    let typeHTML = `<code className="m-0 ml-4 pb-0 pt-0.5 text-cream-500"><span className="font-normal text-cream-600">${
      parameter.required ? 'required' : 'optional'
    }</span> ${parameter.in == 'path' ? 'path parameter' : 'query parameter'}</code>`;
    parameterHTML += `<div className="flex flex-row items-center my-2">\n<h2 className="text-lg font-bold text-cream-50 m-0">${parameter.name.replace(
      '_',
      '\\_'
    )}</h2>${typeHTML}\n</div>\n`;

    if (parameter.description || parameter.schema.description) {
      parameterHTML += `<div className="description pl-2">\n${
        parameter.description || parameter.schema.description
      }\n</div>\n`;
    }

    parameterHTML += `\n</div>\n`;
  }

  return parameterHTML;
}

export function convertJsonSchemaToSchema(schema) {
  if (schema?.type === 'object') {
    const properties = Object.fromEntries(
      Object.entries(schema.properties || {}).map(([key, value]) => [key, convertJsonSchemaToSchema(value)])
    );

    if (schema.additionalProperties) {
      return {
        type: 'union',
        description: schema.description,
        items: [
          {
            type: 'object',
            description: schema.description,
            properties: properties
          },
          convertJsonSchemaToSchema(schema.additionalProperties)
        ]
      };
    }

    return {
      type: 'object',
      description: schema.description,
      properties: properties
    };
  }

  if (schema?.type === 'array') {
    // console.log(schema);
    return {
      type: 'array',
      description: schema.description,
      item: convertJsonSchemaToSchema(schema.items)
    };
  }

  if (schema?.type === 'string') {
    return {
      type: 'string',
      description: schema.description
    };
  }

  if (schema?.type === 'integer') {
    return {
      type: 'number',
      description: schema.description
    };
  }

  if (schema?.type === 'number') {
    return {
      type: 'number',
      description: schema.description
    };
  }

  if (schema?.type === 'boolean') {
    return {
      type: 'boolean',
      description: schema.description
    };
  }

  if (!schema) {
    return {
      type: 'any'
    };
  }
  console.log('!!!!!!Unknown schema type:', schema);
  return {
    type: 'any'
  };
}
