import { Heading } from '@/components/Heading';
import NextImage from 'next/image';
import Link from 'next/link';
import { SchemaPreview as Schema } from '@/components/SchemaPreview';

export const a = Link;

export const Image = props => <NextImage {...props} />;

export const h2 = function H2(props) {
  return <Heading level={2} {...props} />;
};

export const h3 = function H3(props) {
  return <Heading level={3} {...props} />;
};

export const table = function Table(props) {
  return (
    <div className='overflow-x-auto'>
      <table {...props} />
    </div>
  );
};

export const SchemaPreview = ({ schema }) => {
  return (
    <div className='not-prose rounded-md border p-4'>
      <Schema schema={schema} />
    </div>
  );
};

export const Lead = ({ children }) => {
  return <p class='mb-10 text-lg font-semibold leading-7'>{children}</p>;
};

export * from '@rivet-gg/components/mdx';
export { Resource } from '@/components/Resources';
export { Summary } from '@/components/Summary';
export { Accordion, AccordionGroup } from '@/components/Accordion';
export { Frame } from '@/components/Frame';
export { Card, CardGroup } from '@/components/Card';
export { pre, code, CodeGroup, Code } from '@/components/v2/Code';
