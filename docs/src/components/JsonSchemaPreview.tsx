// Renders JSON schema (draft 7)

import { Foldable } from '@/components/FoldableSchema';
import { Markdown } from '@/components/Markdown';
import { cn } from '@rivet-gg/components';
import { ReactNode } from 'react';
import type { JSONSchema7, JSONSchema7Definition, JSONSchema7Type } from 'json-schema';

interface JsonSchemaPreviewProps {
  className?: string;
  schema: JSONSchema7;
  defs?: Record<string, JSONSchema7>;
  parent?: string;
  empty?: ReactNode;
}

export function JsonSchemaPreview({ className, schema, defs, parent, empty }: JsonSchemaPreviewProps) {
  if (schema.type === 'object') {
    if (!schema.properties || Object.keys(schema.properties).length === 0) {
      return empty;
    }

    if (schema.title) {
      return (
        <div className='not-prose mb-6 rounded-md border px-4 pb-3'>
          <h3 className='relative -top-4 mb-0 inline-block bg-card text-xl font-bold'>{schema.title}</h3>
          <ObjectSchema
            className={className}
            schema={schema}
            defs={defs ?? (schema.definitions as Record<string, JSONSchema7>) ?? {}}
            parent={parent}
          />
        </div>
      );
    } else {
      return (
        <ObjectSchema
          className={className}
          schema={schema}
          defs={defs ?? (schema.definitions as Record<string, JSONSchema7>) ?? {}}
          parent={parent}
        />
      );
    }
  }

  return null;
}

interface ObjectSchemaProps {
  schema: JSONSchema7;
  defs: Record<string, JSONSchema7>;
  parent?: string;
  className?: string;
}

function ObjectSchema({ schema: baseSchema, defs, parent, className }: ObjectSchemaProps) {
  let schema = resolveSchema(baseSchema, defs);

  return (
    <ul className={cn({ 'rounded-md border py-4': !!parent }, 'space-y-4', className)}>
      {Object.entries(schema.properties ?? {}).map(([key, property]) => {
        let resolved = resolveSchema(property as JSONSchema7, defs);
        const nullable = !schema.required?.some(r => r == key);
        let newParent = parent ? `${parent}.${key}` : key;

        return (
          <ObjectSchemaItem key={key}>
            <PropertyLabel parent={parent} name={key} schema={resolved} nullable={nullable} />
            <Schema parent={newParent} schema={resolved} defs={defs} />
          </ObjectSchemaItem>
        );
      })}
    </ul>
  );
}

interface SchemaProps {
  schema: JSONSchema7;
  defs: Record<string, JSONSchema7>;
  parent?: string;
  className?: string;
  foldable?: boolean;
}

function Schema({ schema: baseSchema, defs, parent, className, foldable }: SchemaProps) {
  let isFoldable = foldable ?? true;
  let schema = resolveSchema(baseSchema as JSONSchema7, defs);

  // Enum
  if (schema.oneOf) {
    let inner = (
      <ul className='space-y-4 rounded-md'>
        {schema.oneOf.map((item: JSONSchema7, index) => {
          return (
            <li key={index} className='my-4'>
              {item.enum ? (
                <TypeLabel type={item.enum[0]} description={item.description} />
              ) : (
                <Schema parent={parent} schema={item} defs={defs} foldable={false} />
              )}
            </li>
          );
        })}
      </ul>
    );

    return (
      <div className='mt-1 px-4'>
        {isFoldable ? (
          <Foldable title='Show possible variants' closeTitle='Hide possible variants'>
            {inner}
          </Foldable>
        ) : (
          inner
        )}
      </div>
    );
  }

  // Map
  if (schema.additionalProperties) {
    const newParent = `${parent}[key]`;

    const item = resolveSchema(schema.additionalProperties as JSONSchema7, defs);
    const isObject = item.type === 'object';
    const isEmpty = item.type === 'object' && (!item.properties || Object.keys(item.properties).length === 0);

    if (isObject && !isEmpty) {
      return (
        <div className='mt-1 px-4'>
          {isFoldable ? (
            <Foldable>
              <JsonSchemaPreview schema={item} defs={defs} parent={newParent} />
            </Foldable>
          ) : (
            <JsonSchemaPreview schema={item} defs={defs} parent={newParent} />
          )}
        </div>
      );
    }
    return null;
  }

  if (schema.type === 'object') {
    const isEmpty = !schema.properties || Object.keys(schema.properties).length === 0;

    if (!isEmpty) {
      return (
        <div className='mt-1 px-4'>
          {isFoldable ? (
            <Foldable>
              <JsonSchemaPreview schema={schema} defs={defs} parent={parent} />
            </Foldable>
          ) : (
            <JsonSchemaPreview schema={schema} defs={defs} parent={parent} />
          )}
        </div>
      );
    }
    return null;
  }

  if (schema.type === 'array' && schema.items) {
    const newParent = `${parent}[]`;

    const items = schema.items as JSONSchema7;
    const isObject = items.type === 'object';
    const isEmpty =
      items.type === 'object' && (!items.properties || Object.keys(items.properties).length === 0);

    if (isObject && !isEmpty) {
      return (
        <div className='mt-1 px-4'>
          {isFoldable ? (
            <Foldable>
              <JsonSchemaPreview schema={items} defs={defs} parent={newParent} />
            </Foldable>
          ) : (
            <JsonSchemaPreview schema={items} defs={defs} parent={newParent} />
          )}
        </div>
      );
    }
    return null;
  }

  if (schema.enum) {
    let inner = (
      <ul className='space-y-4 rounded-md'>
        {schema.enum.map((item, index) => {
          return (
            <li key={index} className='my-4'>
              <TypeLabel type={item} />
            </li>
          );
        })}
      </ul>
    );
    return (
      <div className='mt-1 px-4'>
        {isFoldable ? (
          <Foldable title='Show possible variants' closeTitle='Hide possible variants'>
            {inner}
          </Foldable>
        ) : (
          inner
        )}
      </div>
    );
  }

  return <JsonSchemaPreview schema={schema} defs={defs} parent={parent} />;
}

function ObjectSchemaItem({ children }) {
  return (
    <li className='min-w-0 overflow-auto whitespace-pre border-b pb-4 last:border-none last:pb-0'>
      {children}
    </li>
  );
}

interface PropertyLabelProps {
  parent?: string;
  name: string;
  schema: JSONSchema7;
  nullable: boolean;
}

function PropertyLabel({ parent, name, schema, nullable }: PropertyLabelProps) {
  return (
    <>
      <div className='scrollbar-hide flex items-center gap-1 overflow-auto px-4'>
        <code className='text-foreground/90'>
          {parent ? <>{parent}.</> : null}
          <span className='font-bold text-foreground'>{name}</span>
        </code>
        <div className='text-xs opacity-20'>{getPropertyTypeLabel(schema, nullable)}</div>
      </div>

      <div className='text-wrap px-4 text-sm text-muted-foreground'>
        <Markdown>{schema.description || ''}</Markdown>
      </div>
    </>
  );
}

interface TypeLabelProps {
  type: JSONSchema7Type;
  description?: string;
}

function TypeLabel({ type, description }: TypeLabelProps) {
  return (
    <>
      <div className='scrollbar-hide flex items-center gap-1 overflow-auto px-4'>
        <code className='font-bold text-foreground'>{getTypeLabel(type)}</code>
      </div>

      <div className='prose text-wrap px-4 text-sm text-muted-foreground'>
        <Markdown>{description || ''}</Markdown>
      </div>
    </>
  );
}
function getPropertyTypeLabel(schema: JSONSchema7, nullable: boolean = false) {
  let s: string[] = [];

  if (nullable) {
    s.push('nullable');
  }

  if (schema.oneOf) {
    let type = Array.from(new Set(schema.oneOf.map((s: JSONSchema7) => getPropertyTypeLabel(s))));
    s.push(type.join(', '));
  } else if (schema.type === 'string') {
    s.push('string');
  } else if (schema.type === 'number') {
    s.push('number');
  } else if (schema.type === 'integer') {
    s.push('integer');
  } else if (schema.type === 'boolean') {
    s.push('boolean');
  } else if (schema.type === 'array') {
    s.push(`array of ${getPropertyTypeLabel(schema.items as JSONSchema7)}s`);
  } else if (schema.type === 'object') {
    if (schema.additionalProperties) {
      s.push('map');
    } else if (!schema.properties || Object.keys(schema.properties).length === 0) {
      s.push('empty object');
    } else {
      s.push('object');
    }
  } else if (schema.type === 'null') {
    s.push('null');
  }

  return s.join(' ');
}

function getTypeLabel(type: JSONSchema7Type) {
  if (type instanceof Array) {
    return `[${type.map(getTypeLabel).join(', ')}]`;
  }
  if (type == null) {
    return 'null';
  }

  if (typeof type == 'object') {
    return `{ ${Object.entries(type)
      .map(([key, type]) => `${key}: ${getTypeLabel(type)}`)
      .join(', ')} }`;
  }

  return type;
}

function resolveSchema(schema: JSONSchema7, defs: Record<string, JSONSchema7Definition>): JSONSchema7 {
  if (schema.allOf?.length) {
    if (schema.allOf.length == 1) {
      return resolveSchema(schema.allOf[0] as JSONSchema7, defs);
    }

    throw new Error('unsupported');
  }

  if (schema.$ref) {
    return defs[schema.$ref.slice('#/definitions/'.length)] as JSONSchema7;
  }

  return schema;
}
