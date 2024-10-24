import * as mdx from '@/components/mdx';

export function useMDXComponents(components) {
  return {
    ...components,
    ...mdx
  };
}
