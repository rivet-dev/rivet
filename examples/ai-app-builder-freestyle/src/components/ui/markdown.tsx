import { cn } from "@/lib/utils";
import { marked } from "marked";
import { memo, useId, useMemo } from "react";
import ReactMarkdown, { Components } from "react-markdown";
import remarkGfm from "remark-gfm";
import { CodeBlock, CodeBlockCode } from "./code-block";

export type MarkdownProps = {
  children: string;
  id?: string;
  className?: string;
  components?: Partial<Components>;
};

function parseMarkdownIntoBlocks(markdown: string): string[] {
  const tokens = marked.lexer(markdown);
  return tokens.map((token) => token.raw);
}

function extractLanguage(className?: string): string {
  if (!className) return "plaintext";
  const match = className.match(/language-(\w+)/);
  return match ? match[1] : "plaintext";
}

// Add tailwind components here
const INITIAL_COMPONENTS: Partial<Components> = {
  // Code blocks with syntax highlighting
  code: function CodeComponent({ className, children, ...props }) {
    const isInline =
      !props.node?.position?.start.line ||
      props.node?.position?.start.line === props.node?.position?.end.line;

    if (isInline) {
      return (
        <span
          className={cn(
            "bg-primary-foreground rounded-sm px-1 font-mono text-sm",
            className,
          )}
          {...props}
        >
          {children}
        </span>
      );
    }

    const language = extractLanguage(className);

    return (
      <CodeBlock className={className}>
        <CodeBlockCode code={children as string} language={language} />
      </CodeBlock>
    );
  },
  pre: function PreComponent({ children }) {
    return <>{children}</>;
  },
  // Make all links open in new tab
  a: function LinkComponent({ className, children, href, ...props }) {
    return (
      <a
        className={cn(
          "text-blue-600 hover:text-blue-800 dark:text-blue-400 dark:hover:text-blue-300 underline underline-offset-2 transition-colors",
          className,
        )}
        href={href}
        target="_blank"
        rel="noopener noreferrer"
        {...props}
      >
        {children}
      </a>
    );
  },
  // Style headings
  h1: ({ className, children, ...props }) => (
    <h1 className={cn("text-2xl font-bold mt-6 mb-4", className)} {...props}>
      {children}
    </h1>
  ),
  h2: ({ className, children, ...props }) => (
    <h2 className={cn("text-xl font-bold mt-5 mb-3", className)} {...props}>
      {children}
    </h2>
  ),
  h3: ({ className, children, ...props }) => (
    <h3 className={cn("text-lg font-semibold mt-4 mb-2", className)} {...props}>
      {children}
    </h3>
  ),
  // Style paragraphs
  p: ({ className, children, ...props }) => (
    <p className={cn("mb-4 leading-relaxed", className)} {...props}>
      {children}
    </p>
  ),
  // Style lists
  ul: ({ className, children, ...props }) => (
    <ul className={cn("list-disc pl-6 mb-4", className)} {...props}>
      {children}
    </ul>
  ),
  ol: ({ className, children, ...props }) => (
    <ol className={cn("list-decimal pl-6 mb-4", className)} {...props}>
      {children}
    </ol>
  ),
  li: ({ className, children, ...props }) => (
    <li className={cn("mb-1", className)} {...props}>
      {children}
    </li>
  ),
  // Style blockquotes
  blockquote: ({ className, children, ...props }) => (
    <blockquote
      className={cn(
        "border-l-4 border-gray-300 dark:border-gray-700 pl-4 italic my-4 text-gray-700 dark:text-gray-300",
        className,
      )}
      {...props}
    >
      {children}
    </blockquote>
  ),
  // Style tables
  table: ({ className, children, ...props }) => (
    <div className="overflow-x-auto mb-4">
      <table
        className={cn(
          "min-w-full divide-y divide-gray-300 dark:divide-gray-700",
          className,
        )}
        {...props}
      >
        {children}
      </table>
    </div>
  ),
  thead: ({ className, children, ...props }) => (
    <thead className={cn("bg-gray-100 dark:bg-gray-800", className)} {...props}>
      {children}
    </thead>
  ),
  tbody: ({ className, children, ...props }) => (
    <tbody
      className={cn("divide-y divide-gray-200 dark:divide-gray-800", className)}
      {...props}
    >
      {children}
    </tbody>
  ),
  tr: ({ className, children, ...props }) => (
    <tr
      className={cn(
        "transition-colors hover:bg-gray-50 dark:hover:bg-gray-800",
        className,
      )}
      {...props}
    >
      {children}
    </tr>
  ),
  th: ({ className, children, ...props }) => (
    <th
      className={cn(
        "px-4 py-3 text-left text-xs font-medium text-gray-700 dark:text-gray-300 uppercase tracking-wider",
        className,
      )}
      {...props}
    >
      {children}
    </th>
  ),
  td: ({ className, children, ...props }) => (
    <td
      className={cn(
        "px-4 py-3 text-sm text-gray-600 dark:text-gray-400",
        className,
      )}
      {...props}
    >
      {children}
    </td>
  ),
  // Style images
  img: ({ className, alt, ...props }) => (
    <img
      className={cn("max-w-full h-auto rounded-md my-4", className)}
      alt={alt}
      {...props}
    />
  ),
  // Style horizontal rules
  hr: ({ className, ...props }) => (
    <hr
      className={cn("my-8 border-gray-300 dark:border-gray-700", className)}
      {...props}
    />
  ),
};

const MemoizedMarkdownBlock = memo(
  function MarkdownBlock({
    content,
    components = INITIAL_COMPONENTS,
  }: {
    content: string;
    components?: Partial<Components>;
  }) {
    return (
      <ReactMarkdown remarkPlugins={[remarkGfm]} components={components}>
        {content}
      </ReactMarkdown>
    );
  },
  function propsAreEqual(prevProps, nextProps) {
    return prevProps.content === nextProps.content;
  },
);

MemoizedMarkdownBlock.displayName = "MemoizedMarkdownBlock";

function MarkdownComponent({
  children,
  id,
  className,
  components = INITIAL_COMPONENTS,
}: MarkdownProps) {
  const generatedId = useId();
  const blockId = id ?? generatedId;
  const blocks = useMemo(() => parseMarkdownIntoBlocks(children), [children]);

  return (
    <div className={className}>
      {blocks.map((block, index) => (
        <MemoizedMarkdownBlock
          key={`${blockId}-block-${index}`}
          content={block}
          components={components}
        />
      ))}
    </div>
  );
}

const Markdown = memo(MarkdownComponent);
Markdown.displayName = "Markdown";

export { Markdown };
