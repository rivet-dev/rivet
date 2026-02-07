import { Heading } from '@/components/Heading';
import { SchemaPreview as Schema } from '@/components/SchemaPreview';

// In Astro, we use regular anchor tags instead of Next.js Link
export const a = (props) => <a {...props} />;

// Handle both string URLs and Astro image objects (which have { src, width, height })
export const img = ({ src, ...props }) => {
  // If src is an object (Astro image import), extract the src property and dimensions
  const isAstroImage = typeof src === 'object' && src !== null;
  const imgSrc = isAstroImage ? src.src : src;
  const width = isAstroImage ? src.width : props.width;
  const height = isAstroImage ? src.height : props.height;

  return (
    <img
      src={imgSrc}
      width={width}
      height={height}
      loading="lazy"
      decoding="async"
      {...props}
    />
  );
};

// Regular img tag for images with optimization attributes
export const Image = ({ src, ...props }) => {
  const isAstroImage = typeof src === 'object' && src !== null;
  const imgSrc = isAstroImage ? src.src : src;
  const width = isAstroImage ? src.width : props.width;
  const height = isAstroImage ? src.height : props.height;

  return (
    <img
      src={imgSrc}
      width={width}
      height={height}
      loading="lazy"
      decoding="async"
      {...props}
    />
  );
};

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
  return <p className='mb-10 text-lg font-semibold leading-7'>{children}</p>;
};

// Re-export from @rivet-gg/components/mdx but override Tabs with SSR-safe version
export {
  Tip,
  Warning,
  Note,
  Info,
  Image as MdxImage,
  Card as MdxCard,
  CardGroup as MdxCardGroup,
  Step,
  Steps,
} from '@rivet-gg/components/mdx';
// Use our local SSR-safe Tabs instead of the one from @rivet-gg/components/mdx
export { Tab, Tabs } from '@/components/Tabs';
export { Resource } from '@/components/Resources';
export { Summary } from '@/components/Summary';
export { Accordion, AccordionGroup } from '@/components/Accordion';
export { Frame } from '@/components/Frame';
export { Card, CardGroup } from '@/components/Card';
export { pre, code, CodeGroup, Code } from '@/components/v2/Code';

// Simple Tooltip component for inline tooltips
export const Tooltip = ({ tip, children }) => {
  return (
    <span className="relative group cursor-help border-b border-dotted border-current">
      {children}
      <span className="absolute bottom-full left-1/2 -translate-x-1/2 mb-1 px-2 py-1 text-xs bg-black text-white rounded opacity-0 group-hover:opacity-100 transition-opacity whitespace-nowrap pointer-events-none z-50">
        {tip}
      </span>
    </span>
  );
};
