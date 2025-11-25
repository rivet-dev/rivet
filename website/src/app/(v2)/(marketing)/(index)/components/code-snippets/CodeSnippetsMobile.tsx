'use client';

import { useState, useEffect } from 'react';
import { Icon, faGithub, faChevronDown, faChevronRight, faCode } from '@rivet-gg/icons';
import { examples, type ExampleData } from '@/data/examples/examples';
import { EXAMPLE_ICON_MAP, createExampleActions } from './utils';
import * as shiki from 'shiki';
import theme from '@/lib/textmate-code-theme';

interface ExampleListItemProps {
  example: ExampleData;
  icon: any;
  isExpanded: boolean;
  onToggle: () => void;
}

let highlighter: shiki.Highlighter;

function ExampleListItem({ example, icon, isExpanded, onToggle }: ExampleListItemProps) {
  const [fileContent, setFileContent] = useState<string>('');
  const [isCodeExpanded, setIsCodeExpanded] = useState<boolean>(false);
  const { handleOpenGithub } = createExampleActions(example.id, example.files);

  // Get the main file to display
  const mainFile = example.filesToOpen[0] || Object.keys(example.files)[0];

  // Reset code expanded state when accordion is collapsed
  useEffect(() => {
    if (!isExpanded) {
      setIsCodeExpanded(false);
    }
  }, [isExpanded]);

  // Initialize highlighter and highlight code when expanded
  useEffect(() => {
    const highlightCode = async () => {
      if (!isExpanded || !mainFile) return;

      highlighter ??= await shiki.getSingletonHighlighter({
        langs: ['typescript', 'json'],
        themes: [theme]
      });

      const code = example.files[mainFile] || '';
      const lang = mainFile.endsWith('.json') ? 'json' : 'typescript';

      const highlighted = highlighter.codeToHtml(code, {
        lang,
        theme: theme.name
      });

      setFileContent(highlighted);
    };

    highlightCode();
  }, [isExpanded, mainFile, example.files]);

  return (
    <div className='overflow-hidden rounded-lg border border-white/15 bg-white/[0.06]'>
      <button
        onClick={onToggle}
        className='flex w-full items-center gap-2.5 p-3 text-left transition-colors hover:bg-white/[0.04]'
      >
        <Icon
          icon={isExpanded ? faChevronDown : faChevronRight}
          className='h-3 w-3 flex-shrink-0 text-white/50'
        />
        <Icon icon={icon} className='h-4 w-4 flex-shrink-0 text-white/70' />
        <div className='min-w-0 flex-1'>
          <h3 className='text-sm font-medium text-white'>{example.title}</h3>
        </div>
      </button>

      {isExpanded && (
        <div className='border-t border-white/15'>
          {/* Code snippet */}
          <div className='relative bg-[#0d0b0a]'>
            <div className='relative'>
              <div
                className={`code overflow-x-auto overflow-y-hidden p-3 text-xs transition-all duration-300 ${
                  isCodeExpanded ? '' : 'max-h-[900px]'
                }`}
                // biome-ignore lint/security/noDangerouslySetInnerHtml: we trust shiki
                dangerouslySetInnerHTML={{ __html: fileContent }}
              />

              {/* Gradient overlay and Show More button */}
              {!isCodeExpanded && (
                <div className='absolute bottom-0 left-0 right-0 flex h-24 items-end justify-center bg-gradient-to-t from-[#0d0b0a] via-[#0d0b0a]/90 to-transparent pb-3'>
                  <button
                    onClick={() => setIsCodeExpanded(true)}
                    className='rounded-md border border-white/20 bg-white/10 px-4 py-1.5 text-xs font-medium text-white/80 transition-all duration-200 hover:border-white/30 hover:bg-white/15 hover:text-white'
                  >
                    Show more
                  </button>
                </div>
              )}
            </div>
          </div>

          {/* GitHub button - only shown when expanded */}
          {/* <div className="p-3 border-t border-white/15 bg-white/[0.02]">
						<button
							onClick={handleOpenGithub}
							className="w-full flex items-center justify-center gap-2 px-3 py-2 text-xs font-medium text-white/80 hover:text-white hover:bg-white/8 border border-white/15 hover:border-white/25 rounded-md transition-all duration-200"
						>
							<Icon icon={faGithub} className="w-3.5 h-3.5" />
							View on GitHub
						</button>
					</div> */}
        </div>
      )}
    </div>
  );
}

export default function CodeSnippetsMobile() {
  const [expandedExamples, setExpandedExamples] = useState<Set<string>>(new Set());

  const toggleExample = (exampleId: string) => {
    setExpandedExamples(prev => {
      const next = new Set(prev);
      if (next.has(exampleId)) {
        next.delete(exampleId);
      } else {
        next.add(exampleId);
      }
      return next;
    });
  };

  const examplesWithIcons = examples.map(example => ({
    ...example,
    icon: EXAMPLE_ICON_MAP[example.id] || faCode
  }));

  return (
    <div>
      <h2 className='mb-4 text-center text-sm font-medium text-white/70'>Examples</h2>
      <div className='space-y-3'>
        {examplesWithIcons.map(example => (
          <ExampleListItem
            key={example.id}
            example={example}
            icon={example.icon}
            isExpanded={expandedExamples.has(example.id)}
            onToggle={() => toggleExample(example.id)}
          />
        ))}
      </div>
    </div>
  );
}
