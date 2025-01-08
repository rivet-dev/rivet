// Renders JSON schema (draft 7)

import { Foldable } from '@/components/FoldableSchema';
import { Markdown } from '@/components/Markdown';
import { cn } from '@rivet-gg/components';
import { ReactNode, ReactElement } from 'react';
import type { JSONSchema7, JSONSchema7Definition, JSONSchema7Type } from 'json-schema';

interface JsonSchemaPreviewProps {
  className?: string;
  title?: string;
  schema: JSONSchema7;
  defs?: Record<string, JSONSchema7>;
  parent?: string;
  empty?: ReactNode;
}

export function JsonSchemaPreview({
  className,
  title,
  schema: baseSchema,
  defs: baseDefs,
  parent,
  empty
}: JsonSchemaPreviewProps) {
  let defs = baseDefs ?? (baseSchema.definitions as Record<string, JSONSchema7>) ?? {};
  let schema = resolveSchema(baseSchema, defs);

  if (schema.type === 'object' || schema.$ref) {
    if (schema.type === 'object' && (!schema.properties || Object.keys(schema.properties).length === 0)) {
      return empty;
    }

    if (title ?? schema.title) {
      return (
        <div className={cn('not-prose mb-6 rounded-md border px-4 pb-4', className)}>
          <h3 className='relative -top-4 mb-0 inline-block bg-card text-xl font-bold'>
            {title ?? schema.title}
          </h3>
          <ObjectSchema schema={schema} defs={defs} parent={parent} />
        </div>
      );
    } else {
      return <ObjectSchema className={className} schema={schema} defs={defs} parent={parent} />;
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
          <ObjectSchemaItem className={'px-4'} key={key}>
            <PropertyLabel parent={parent} name={key} schema={resolved} defs={defs} nullable={nullable} />
            <Schema parent={newParent} schema={resolved} defs={defs} />
          </ObjectSchemaItem>
        );
      })}
      {schema.anyOf ? (
        <li className='px-4'>
          <p className='mb-2'>Any of the following variants:</p>
          <ul className='space-y-4 rounded-md border py-4'>
            {schema.anyOf?.map((ref: JSONSchema7, index) => {
              return (
                <ObjectSchemaItem className='px-4' key={ref.$ref}>
                  <p>Variant #{index + 1}</p>
                  <Schema parent={parent} schema={ref as JSONSchema7} defs={defs} />
                </ObjectSchemaItem>
              );
            })}
          </ul>
        </li>
      ) : null}
    </ul>
  );
}

interface SchemaProps {
  schema: JSONSchema7;
  defs: Record<string, JSONSchema7>;
  parent?: string;
  foldable?: boolean;
}

function Schema({ schema: baseSchema, defs, parent, foldable }: SchemaProps) {
  let isFoldable = foldable ?? true;
  let schema = resolveSchema(baseSchema as JSONSchema7, defs);

  // Enum
  if (schema.oneOf) {
    const common = {
      type: 'object',
      properties: schema.properties
    } as JSONSchema7;

    let inner = (
      <ul>
        {schema.oneOf.map((item: JSONSchema7, index) => {
          return (
            <li key={index} className='mt-4 pl-4'>
              {item.enum ? (
                <TypeLabel type={item.enum[0]} description={item.description} />
              ) : (
                <>
                  <TypeLabel type={`Variant #${index + 1}`} />
                  <Schema parent={parent} schema={item} defs={defs} foldable={false} />
                </>
              )}
            </li>
          );
        })}
        {common.properties ? (
          <li className='mt-4 pl-4'>
            <TypeLabel type={'Common (on all variants)'} />
            <Schema parent={parent} schema={common} defs={defs} foldable={false} />
          </li>
        ) : null}
      </ul>
    );

    return isFoldable ? (
      <Foldable title='Show possible variants' closeTitle='Hide possible variants'>
        {inner}
      </Foldable>
    ) : (
      inner
    );
  }

  // String enum
  if (schema.enum) {
    let inner = (
      <ul className='space-y-4 rounded-md'>
        {schema.enum.map((item, index) => {
          return (
            <li key={index} className='mt-4 px-4'>
              <TypeLabel type={item} />
            </li>
          );
        })}
      </ul>
    );
    return isFoldable ? (
      <Foldable title='Show possible variants' closeTitle='Hide possible variants'>
        {inner}
      </Foldable>
    ) : (
      inner
    );
  }

  // Map
  if (schema.additionalProperties) {
    const newParent = `${parent}[key]`;

    const item = resolveSchema(schema.additionalProperties as JSONSchema7, defs);
    const isObject = item.type === 'object';
    const isEmpty = item.type === 'object' && (!item.properties || Object.keys(item.properties).length === 0);

    if (isObject && !isEmpty) {
      return isFoldable ? (
        <Foldable>
          <JsonSchemaPreview schema={item} defs={defs} parent={newParent} />
        </Foldable>
      ) : (
        <JsonSchemaPreview schema={item} defs={defs} parent={newParent} />
      );
    }
    return null;
  }

  // Object
  if (schema.type === 'object') {
    const isEmpty = !schema.properties || Object.keys(schema.properties).length === 0;

    if (!isEmpty) {
      return isFoldable ? (
        <Foldable>
          <ObjectSchema schema={schema} defs={defs} parent={parent} />
        </Foldable>
      ) : (
        <ObjectSchema schema={schema} defs={defs} parent={parent} />
      );
    }
    return null;
  }

  // Array
  if (schema.type === 'array' && schema.items) {
    const newParent = `${parent}[]`;

    const items = resolveSchema(schema.items as JSONSchema7, defs);
    const isObject = items.type === 'object';
    const isEmpty =
      items.type === 'object' && (!items.properties || Object.keys(items.properties).length === 0);
    const isEnum = items.oneOf != undefined;

    if (isObject && !isEmpty) {
      return isFoldable ? (
        <Foldable
          title={isEnum ? 'Show possible variants' : undefined}
          closeTitle={isEnum ? 'Hide possible variants' : undefined}>
          <Schema schema={items} defs={defs} parent={newParent} foldable={false} />
        </Foldable>
      ) : (
        <Schema schema={items} defs={defs} parent={newParent} />
      );
    }
    return null;
  }

  return null;
}

interface ObjectSchemaItemProps {
  className?: string;
  children: ReactElement[];
}

function ObjectSchemaItem({ children, className }: ObjectSchemaItemProps) {
  return (
    <li
      className={cn(
        'min-w-0 overflow-auto whitespace-pre border-b pb-4 last:border-none last:pb-0',
        className
      )}>
      {children}
    </li>
  );
}

interface PropertyLabelProps {
  className?: string;
  parent?: string;
  name: string;
  schema: JSONSchema7;
  defs: Record<string, JSONSchema7>;
  nullable: boolean;
}

export function PropertyLabel({ className, parent, name, schema, defs, nullable }: PropertyLabelProps) {
  return (
    <>
      <div className={cn('scrollbar-hide flex items-center gap-1 overflow-auto', className)}>
        <code className='text-foreground/90'>
          {parent ? <>{parent}.</> : null}
          <span className='font-bold text-foreground'>{name}</span>
        </code>
        <div className='text-xs opacity-20'>{getPropertyTypeLabel(schema, defs, nullable)}</div>
      </div>
      <div className='prose text-wrap text-sm text-muted-foreground'>
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
      <div className='scrollbar-hide flex items-center gap-1 overflow-auto'>
        <code className='font-bold text-foreground'>{getTypeLabel(type)}</code>
      </div>

      <div className='prose text-wrap text-sm text-muted-foreground'>
        <Markdown>{description || ''}</Markdown>
      </div>
    </>
  );
}
function getPropertyTypeLabel(
  schema: JSONSchema7,
  defs: Record<string, JSONSchema7>,
  nullable: boolean = false
) {
  let s: string[] = [];

  if (nullable) {
    s.push('nullable');
  }

  if (schema.oneOf) {
    let type = Array.from(new Set(schema.oneOf.map((s: JSONSchema7) => getPropertyTypeLabel(s, defs))));
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
    s.push(`array of ${getPropertyTypeLabel(resolveSchema(schema.items as JSONSchema7, defs), defs)}s`);
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
  } else {
    s.push('unknown');
  }

  // Location from OpenAPI spec
  if ((schema as any).in) {
    s.push(`${(schema as any).in} parameter`);
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
    let cleanRef = schema.$ref.replace(/#\/(definitions|components\/schemas)\//, '');
    return defs[cleanRef] as JSONSchema7;
  }

  return schema;
}
