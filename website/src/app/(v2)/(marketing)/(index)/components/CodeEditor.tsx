'use client';

import { useState, useEffect, useRef } from 'react';
import sdk, { VM, Project } from '@stackblitz/sdk';
import { examples, type ExampleData, type StateTypeTab } from '@/data/examples/examples';

interface CodeEditorProps {
  activeExample: string;
  activeStateType: StateTypeTab;
}

export default function CodeEditor({ activeExample, activeStateType }: CodeEditorProps) {
  const [vm, setVm] = useState<VM | null>(null);
  const embedRef = useRef<HTMLDivElement>(null);
  const initializedRef = useRef<boolean>(false);

  // Create StackBlitz project from static examples
  const createProject = (example: string, state: StateTypeTab): Project => {
    const exampleData = examples.find(ex => ex.id === example)!;
    return {
      title: `Rivet ${exampleData.title} Example`,
      description: `A ${exampleData.title} example using Rivet Actors`,
      template: 'node',
      files: exampleData.files,
      settings: {
        compile: {
          clearConsole: false
        }
      }
    };
  };

  // Get files to open for the current example and state
  const getExample = (example: string): ExampleData => {
    return examples.find(ex => ex.id === example)!;
  };

  // Initialize StackBlitz embed
  useEffect(() => {
    const element = embedRef.current;
    if (!element) return;

    const embedProject = async () => {
      try {
        // Clear the container before embedding
        element.innerHTML = '';

        // Reset initialization flag
        initializedRef.current = false;

        const project = createProject(activeExample, activeStateType);
        const example = getExample(activeExample);
        const stackblitzVm = await sdk.embedProject(element, project, {
          openFile: example.filesToOpen[0] || 'package.json',
          view: 'editor',
          theme: 'dark',
          height: 600,
          hideNavigation: true,
          hideDevTools: true,
          hideExplorer: false,
          forceEmbedLayout: true
        });

        initializedRef.current = true;
        setVm(stackblitzVm);
      } catch (error) {
        console.error('StackBlitz VM loading error:', error);
      }
    };

    embedProject();
  }, [activeExample, activeStateType]);

  return (
    <div className='h-[600px] w-full'>
      <div
        ref={embedRef}
        className='h-full w-full overflow-hidden rounded-b-2xl'
        style={{ minHeight: '600px' }}
      />
    </div>
  );
}
