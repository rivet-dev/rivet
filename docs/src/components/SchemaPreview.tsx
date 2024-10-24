import { Foldable } from '@/components/FoldableSchema';
import { Markdown } from '@/components/Markdown';
import { cn } from '@rivet-gg/components';
import { ReactNode } from 'react';

type CommonSchema = { description?: string };

type ObjectSchema = { type: 'object'; properties: Record<string, Schema> };
type ArraySchema = { type: 'array'; item: Schema };
type UnionSchema = { type: 'union'; items: Schema[] };
type LiteralSchema = { type: 'literal'; value: any };
type OptionalSchema = { type: 'optional'; value: Schema };
type NullableSchema = { type: 'nullable'; item: Schema };
type RecordSchema = { type: 'record'; elementType: Schema };
type IntersectionSchema = { type: 'intersection'; left: Schema; right: Schema };
type UnknownSchema = { type: 'unknown' };
type StringSchema = { type: 'string' };
type NumberSchema = { type: 'number' };
type BooleanSchema = { type: 'boolean' };
type AnySchema = { type: 'any' };
type DateSchema = { type: 'date' };
type NeverSchema = { type: 'never' };

type Schema = (
  | ObjectSchema
  | ArraySchema
  | UnionSchema
  | LiteralSchema
  | OptionalSchema
  | NullableSchema
  | RecordSchema
  | UnknownSchema
  | StringSchema
  | NumberSchema
  | BooleanSchema
  | AnySchema
  | DateSchema
  | NeverSchema
  | IntersectionSchema
) &
  CommonSchema;

function getPropertyTypeLabel(schema: Schema) {
  if (schema.type === 'array') {
    return `array of ${getPropertyTypeLabel(schema.item)}s`;
  }
  if (schema.type === 'object') {
    if (Object.keys(schema.properties).length === 0) {
      return 'empty object';
    }
    return 'object';
  }
  if (schema.type === 'string') {
    return 'string';
  }
  if (schema.type === 'number') {
    return 'number';
  }
  if (schema.type === 'boolean') {
    return 'boolean';
  }
  if (schema.type === 'union') {
    const types = schema.items.map(getPropertyTypeLabel);
    // count unique types
    const uniqueTypes = [...new Set(types)].map(type => {
      if (types.filter(t => t === type).length > 1) {
        return type + 's';
      }
      return type;
    });

    return `union of ${uniqueTypes.join(' and ')}`;
  }
  if (schema.type === 'literal') {
    return 'literal';
  }
  if (schema.type === 'optional') {
    return `optional ${getPropertyTypeLabel(schema.value)}`;
  }
  if (schema.type === 'nullable') {
    return `nullable ${getPropertyTypeLabel(schema.item)}`;
  }
  if (schema.type === 'record') {
    return `record of ${getPropertyTypeLabel(schema.elementType)}s`;
  }
  if (schema.type === 'unknown') {
    return 'object';
  }
  if (schema.type === 'any') {
    return 'any value';
  }
  if (schema.type === 'date') {
    return 'date';
  }

  if (schema.type === 'intersection') {
    return `combination of ${getPropertyTypeLabel(schema.left)} and ${getPropertyTypeLabel(schema.right)}`;
  }
  console.log('Unknown schema type', schema);
}

type PropertyTypeLabelProps = Schema;

function PropertyTypeLabel(props: PropertyTypeLabelProps) {
  return <div className='text-xs opacity-20'>{getPropertyTypeLabel(props)}</div>;
}

type PropertyLabelProps = Schema & {
  parent?: string;
  name: string;
};

function PropertyLabel({ parent, name, ...rest }: PropertyLabelProps) {
  return (
    <>
      <div className='flex items-center gap-1 px-4'>
        <code className='text-foreground/90'>
          {parent ? <>{parent}.</> : null}
          <span className='text-foreground font-bold'>{name}</span>
        </code>
        <PropertyTypeLabel {...rest} />
      </div>

      <div className='text-muted-foreground px-4 text-sm'>
        <Markdown>{rest.description || ''}</Markdown>
      </div>
    </>
  );
}

function ObjectSchemaItem({ children }) {
  return (
    <li className='min-w-0 overflow-auto whitespace-pre border-b pb-4 last:border-none last:pb-0'>
      {children}
    </li>
  );
}

interface ObjectSchemaProps {
  schema: ObjectSchema;
  parent?: string;
  className?: string;
}
function ObjectSchema({ schema, parent, className }: ObjectSchemaProps) {
  return (
    <ul className={cn({ 'rounded-md border py-4': !!parent }, 'space-y-4', className)}>
      {Object.keys(schema.properties).map(key => {
        let property = schema.properties[key];
        property = property.type === 'nullable' ? property.item : property;
        property = property.type === 'optional' ? property.value : property;

        const newParent = parent ? `${parent}.${key}` : key;

        if (property.type === 'union') {
          return (
            <ObjectSchemaItem key={key}>
              <PropertyLabel parent={parent} name={key} {...schema.properties[key]} />

              <div className='px-4'>
                <Foldable title='Show possible variants' closeTitle='Hide possible variants'>
                  <ul className='space-y-4 rounded-md'>
                    {property.items.map((item, index) => (
                      <li key={index} className='my-4'>
                        <SchemaPreview schema={item} parent={newParent} />
                      </li>
                    ))}
                  </ul>
                </Foldable>
              </div>
            </ObjectSchemaItem>
          );
        }

        if (property.type === 'intersection') {
          const objectSchema = property.left.type === 'object' ? property.left : property.right;
          const isObject = objectSchema.type === 'object';
          const isEmpty = isObject && Object.keys(objectSchema.properties).length === 0;

          return (
            <ObjectSchemaItem key={key}>
              <PropertyLabel parent={parent} name={key} {...schema.properties[key]} />
              {!isEmpty ? (
                <div className='px-4'>
                  <Foldable>
                    <SchemaPreview schema={objectSchema} parent={newParent} />
                  </Foldable>
                </div>
              ) : null}
            </ObjectSchemaItem>
          );
        }

        if (property.type === 'object') {
          const isEmpty = Object.keys(property.properties).length === 0;
          return (
            <ObjectSchemaItem key={key}>
              <PropertyLabel parent={parent} name={key} {...schema.properties[key]} />
              {!isEmpty ? (
                <div className='px-4'>
                  <Foldable>
                    <SchemaPreview schema={property} parent={newParent} />
                  </Foldable>
                </div>
              ) : null}
            </ObjectSchemaItem>
          );
        }
        if (property.type === 'record' && property.elementType.type === 'object') {
          const isEmpty =
            property.elementType.type === 'object' &&
            Object.keys(property.elementType.properties).length === 0;
          return (
            <ObjectSchemaItem key={key}>
              <PropertyLabel parent={parent} name={key} {...schema.properties[key]} />
              {!isEmpty ? (
                <div className='px-4'>
                  <Foldable>
                    <SchemaPreview schema={property.elementType} parent={newParent} />
                  </Foldable>
                </div>
              ) : null}
            </ObjectSchemaItem>
          );
        }

        if (property.type === 'array') {
          const isObject = property.item.type === 'object';
          const isEmpty =
            property.item.type === 'object' && Object.keys(property.item.properties).length === 0;

          return (
            <ObjectSchemaItem key={key}>
              <PropertyLabel parent={parent} name={key} {...schema.properties[key]} />
              {isObject && !isEmpty ? (
                <div className='px-4'>
                  <Foldable>
                    <SchemaPreview schema={property.item} parent={newParent} />
                  </Foldable>
                </div>
              ) : null}
            </ObjectSchemaItem>
          );
        }

        return (
          <ObjectSchemaItem key={key}>
            <PropertyLabel parent={parent} name={key} {...schema.properties[key]} />
            <SchemaPreview schema={property} parent={newParent} />
          </ObjectSchemaItem>
        );
      })}
    </ul>
  );
}

interface SchemaPreviewProps {
  className?: string;
  schema: Schema;
  parent?: string;
  empty?: ReactNode;
}
export function SchemaPreview({ className, schema, parent, empty }: SchemaPreviewProps) {
  if (schema.type === 'object') {
    if (Object.keys(schema.properties).length === 0) {
      return empty;
    }
    return <ObjectSchema className={className} schema={schema} parent={parent} />;
  }
  if (schema.type === 'array') {
    if (Object.keys(schema.item).length === 0) {
      return empty;
    }
    return <SchemaPreview className={className} schema={schema.item} parent={parent} />;
  }
  if (schema.type === 'any') {
    return empty;
  }
  if (schema.type === 'never') {
    return empty;
  }
  return null;
}
