import ReactMarkdown from 'react-markdown';

import * as components from '@/components/mdx';
import { remarkPlugins } from '../mdx/remark.mjs';
import { ComponentProps } from 'react';

type ReactMarkdownProps = ComponentProps<typeof ReactMarkdown>;

export function Markdown({ children }: { children: string }) {
  return (
    <ReactMarkdown
      remarkPlugins={[remarkPlugins] as ReactMarkdownProps['remarkPlugins']}
      components={components as ReactMarkdownProps['components']}>
      {children}
    </ReactMarkdown>
  );
}
