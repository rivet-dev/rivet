'use client';

import { useState, useEffect, useRef } from 'react';
import {
  faFolder,
  faFolderOpen,
  faChevronRight,
  faChevronDown,
  faGithub,
  faFileZip,
  faBolt,
  faCopy,
  faCheck,
  faCode,
  Icon
} from '@rivet-gg/icons';
import { examples, type ExampleData, type StateTypeTab } from '@/data/examples/examples';
import * as shiki from 'shiki';
import theme from '@/lib/textmate-code-theme';
import clsx from 'clsx';
import { EXAMPLE_ICON_MAP, createExampleActions, getFileIcon } from './utils';

const EDITOR_HEIGHT = 800;

interface FileTreeNode {
  name: string;
  path: string;
  type: 'file' | 'folder';
  children?: FileTreeNode[];
  isOpen?: boolean;
}

function buildFileTree(files: string[], preserveOpenState?: FileTreeNode[]): FileTreeNode[] {
  const root: FileTreeNode[] = [];

  // Create a map of existing open states
  const openStateMap = new Map<string, boolean>();
  if (preserveOpenState) {
    const collectOpenStates = (nodes: FileTreeNode[]) => {
      nodes.forEach(node => {
        if (node.type === 'folder' && node.isOpen !== undefined) {
          openStateMap.set(node.path, node.isOpen);
        }
        if (node.children) {
          collectOpenStates(node.children);
        }
      });
    };
    collectOpenStates(preserveOpenState);
  }

  for (const filePath of files) {
    const parts = filePath.split('/');
    let currentLevel = root;

    for (let i = 0; i < parts.length; i++) {
      const part = parts[i];
      const isFile = i === parts.length - 1;
      const currentPath = parts.slice(0, i + 1).join('/');

      let existingNode = currentLevel.find(node => node.name === part);

      if (!existingNode) {
        // Determine if folder should be open by default
        let shouldBeOpen = false;
        if (!isFile) {
          // Check if we have a preserved state
          if (openStateMap.has(currentPath)) {
            shouldBeOpen = openStateMap.get(currentPath)!;
          } else {
            // Default open logic: open frontend & backend
            console.log('path', currentPath);
            shouldBeOpen =
              currentPath === 'src' || currentPath === 'src/frontend' || currentPath === 'src/backend';
          }
        }

        existingNode = {
          name: part,
          path: currentPath,
          type: isFile ? 'file' : 'folder',
          children: isFile ? undefined : [],
          isOpen: isFile ? undefined : shouldBeOpen
        };
        currentLevel.push(existingNode);
      }

      if (!isFile && existingNode.children) {
        currentLevel = existingNode.children;
      }
    }
  }

  return root.sort((a, b) => {
    if (a.type !== b.type) {
      return a.type === 'folder' ? -1 : 1;
    }
    return a.name.localeCompare(b.name);
  });
}

interface FileTreeItemProps {
  node: FileTreeNode;
  depth: number;
  activeFile: string;
  onFileClick: (filePath: string) => void;
  onFolderToggle: (folderPath: string) => void;
}

function FileTreeItem({ node, depth, activeFile, onFileClick, onFolderToggle }: FileTreeItemProps) {
  const isActive = activeFile === node.path;
  const indentSize = depth * 16;

  return (
    <div>
      <button
        onClick={() => {
          if (node.type === 'file') {
            onFileClick(node.path);
          } else {
            onFolderToggle(node.path);
          }
        }}
        className={`flex w-full items-center gap-1 rounded px-2 py-1 text-left text-xs transition-colors ${
          isActive && node.type === 'file'
            ? 'bg-white/15 text-white/80'
            : 'hover:bg-white/8 text-white/60 hover:text-white/80'
        }`}
        style={{ paddingLeft: `${8 + indentSize}px` }}
      >
        {node.type === 'folder' && (
          <Icon icon={node.isOpen ? faChevronDown : faChevronRight} className='h-3 w-3 text-white/50' />
        )}
        <Icon
          icon={node.type === 'folder' ? (node.isOpen ? faFolderOpen : faFolder) : getFileIcon(node.name)}
          className='h-3 w-3 text-white/60'
        />
        <span className='truncate'>{node.name}</span>
      </button>

      {node.type === 'folder' && node.isOpen && node.children && (
        <div>
          {node.children.map(child => (
            <FileTreeItem
              key={child.path}
              node={child}
              depth={depth + 1}
              activeFile={activeFile}
              onFileClick={onFileClick}
              onFolderToggle={onFolderToggle}
            />
          ))}
        </div>
      )}
    </div>
  );
}

interface TabProps {
  children: React.ReactNode;
  isActive: boolean;
  onClick: () => void;
  className?: string;
}

function Tab({ children, isActive, onClick, className = '' }: TabProps) {
  const buttonRef = useRef<HTMLButtonElement>(null);
  const [mousePosition, setMousePosition] = useState({ x: 0, y: 0 });

  const handleMouseMove = (e: React.MouseEvent<HTMLButtonElement>) => {
    if (!buttonRef.current) return;
    const rect = buttonRef.current.getBoundingClientRect();
    setMousePosition({
      x: e.clientX - rect.left,
      y: e.clientY - rect.top
    });
  };

  return (
    <button
      ref={buttonRef}
      onClick={onClick}
      onMouseMove={handleMouseMove}
      className={clsx(
        `group relative flex flex-1 flex-col items-center justify-center gap-2 overflow-hidden whitespace-nowrap rounded-xl border px-4 py-3 text-xs font-medium transition-all duration-200`,
        isActive
          ? 'border-white/10 bg-white/[0.15] text-white'
          : 'border-white/[0.08] bg-white/[0.05] text-white/70 hover:border-white/20 hover:bg-white/[0.08] hover:text-white',
        className
      )}
      style={
        {
          '--mouse-x': `${mousePosition.x}px`,
          '--mouse-y': `${mousePosition.y}px`
        } as React.CSSProperties
      }
    >
      {/* Spotlight effect */}
      <div
        className='pointer-events-none absolute inset-0 opacity-0 transition-opacity duration-300 group-hover:opacity-100'
        style={{
          background: `radial-gradient(circle 120px at var(--mouse-x) var(--mouse-y), rgba(255, 255, 255, 0.08), transparent 80%)`
        }}
      />
      <div className='relative z-10 flex flex-col items-center justify-center gap-2'>{children}</div>
    </button>
  );
}

interface TabGroupProps {
  examples: ExampleData[];
  activeExample: string;
  setActiveExample: (example: string) => void;
  activeStateType: StateTypeTab;
  setActiveStateType: (state: StateTypeTab) => void;
}

function TabGroup({
  examples,
  activeExample,
  setActiveExample,
  activeStateType,
  setActiveStateType
}: TabGroupProps) {
  // Transform examples data to include actual icon components
  const examplesWithIcons = examples.map(example => ({
    ...example,
    icon: EXAMPLE_ICON_MAP[example.id] || faCode
  }));

  return (
    <div className='grid grid-cols-[repeat(auto-fit,minmax(120px,1fr))] gap-2'>
      {examplesWithIcons.map(example => (
        <Tab
          key={example.id}
          isActive={activeExample === example.id}
          onClick={() => setActiveExample(example.id)}
        >
          <Icon icon={example.icon as any} className='h-3.5 w-3.5' />
          {example.title}
        </Tab>
      ))}
    </div>
  );
}

let highlighter: shiki.Highlighter;

interface BottomBarButtonProps {
  onClick: () => void;
  icon: any;
  children: React.ReactNode;
}

function BottomBarButton({ onClick, icon, children }: BottomBarButtonProps) {
  return (
    <button
      onClick={onClick}
      className='hover:bg-white/8 flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-xs font-normal text-white/70 transition-all duration-200 hover:text-white/90'
    >
      <Icon icon={icon} className='h-3 w-3' />
      {children}
    </button>
  );
}

interface BottomBarProps {
  activeExample: string;
}

function BottomBar({ activeExample }: BottomBarProps) {
  const exampleData = examples.find(ex => ex.id === activeExample)!;
  const { handleOpenGithub, handleOpenStackBlitz, handleDownloadZip } = createExampleActions(
    activeExample,
    exampleData.files
  );

  return (
    <div className='flex items-center justify-start'>
      {/* <BottomBarButton onClick={handleOpenGithub} icon={faGithub}>
				View on GitHub
			</BottomBarButton>
			<BottomBarButton onClick={handleDownloadZip} icon={faFileZip}>
				Download ZIP
			</BottomBarButton>
			<BottomBarButton onClick={handleOpenStackBlitz} icon={faBolt}>
				Open in StackBlitz
			</BottomBarButton> */}
    </div>
  );
}

interface CodeEditorProps {
  activeExample: string;
  activeStateType: StateTypeTab;
}

function CodeEditor({ activeExample, activeStateType }: CodeEditorProps) {
  const [activeFile, setActiveFile] = useState<string>('');
  const [fileContent, setFileContent] = useState<string>('');
  const [fileTree, setFileTree] = useState<FileTreeNode[]>([]);
  const [copied, setCopied] = useState<boolean>(false);

  const exampleData = examples.find(ex => ex.id === activeExample)!;
  const files = Object.keys(exampleData.files).filter(
    file =>
      file.endsWith('.ts') ||
      file.endsWith('.tsx') ||
      file.endsWith('.js') ||
      file.endsWith('.jsx') ||
      file.endsWith('.json')
  );

  // Build file tree and set default file on example change
  useEffect(() => {
    const tree = buildFileTree(files, fileTree);
    setFileTree(tree);

    if (files.length > 0) {
      const defaultFile = exampleData.filesToOpen[0] || files[0];
      setActiveFile(defaultFile);
    }
  }, [activeExample]); // Only depend on activeExample to avoid infinite loop

  const handleFolderToggle = (folderPath: string) => {
    const toggleFolder = (nodes: FileTreeNode[]): FileTreeNode[] => {
      return nodes.map(node => {
        if (node.path === folderPath && node.type === 'folder') {
          return { ...node, isOpen: !node.isOpen };
        }
        if (node.children) {
          return { ...node, children: toggleFolder(node.children) };
        }
        return node;
      });
    };

    setFileTree(toggleFolder(fileTree));
  };

  const handleFileClick = (filePath: string) => {
    setActiveFile(filePath);
  };

  const handleCopyCode = async () => {
    if (!activeFile) return;

    const code = exampleData.files[activeFile] || '';
    try {
      await navigator.clipboard.writeText(code);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error('Failed to copy code:', err);
    }
  };

  // Initialize highlighter and highlight code
  useEffect(() => {
    const highlightCode = async () => {
      if (!activeFile) return;

      highlighter ??= await shiki.getSingletonHighlighter({
        langs: ['typescript', 'json'],
        themes: [theme]
      });

      const code = exampleData.files[activeFile] || '';
      const lang = activeFile.endsWith('.json') ? 'json' : 'typescript';

      const highlighted = highlighter.codeToHtml(code, {
        lang,
        theme: theme.name
      });

      setFileContent(highlighted);
    };

    highlightCode();
  }, [activeFile, activeExample]);

  return (
    <div className={`h-[${EDITOR_HEIGHT}px] flex w-full`}>
      {/* Left sidebar - File tree */}
      <div className='w-[160px] flex-shrink-0 border-r border-white/15 bg-[#0d0b0a]'>
        <div className='h-full overflow-auto p-2'>
          {fileTree.map(node => (
            <FileTreeItem
              key={node.path}
              node={node}
              depth={0}
              activeFile={activeFile}
              onFileClick={handleFileClick}
              onFolderToggle={handleFolderToggle}
            />
          ))}
        </div>
      </div>

      {/* Right side - Code viewer */}
      <div className='group relative flex-1'>
        {/* Copy button */}
        <button
          onClick={handleCopyCode}
          className='absolute right-3 top-3 z-10 flex h-8 w-8 items-center justify-center rounded-md border border-white/10 bg-black/30 opacity-0 transition-all duration-200 hover:border-white/20 hover:bg-black/50 group-hover:opacity-100'
          title={copied ? 'Copied!' : 'Copy code'}
        >
          <Icon
            icon={copied ? faCheck : faCopy}
            className={`h-3.5 w-3.5 ${copied ? 'text-green-400' : 'text-white/80'}`}
          />
        </button>
        <div className='h-full overflow-auto bg-[#0d0b0a] [&::-webkit-scrollbar-thumb]:rounded [&::-webkit-scrollbar-thumb]:bg-white/70 [&::-webkit-scrollbar-track]:bg-transparent [&::-webkit-scrollbar]:w-1'>
          <div
            className='code p-4 text-xs'
            // biome-ignore lint/security/noDangerouslySetInnerHtml: we trust shiki
            dangerouslySetInnerHTML={{ __html: fileContent }}
          />
        </div>
      </div>
    </div>
  );
}

interface CodeSnippetsDesktopProps {
  activeExample: string;
  setActiveExample: (example: string) => void;
  activeStateType: StateTypeTab;
  setActiveStateType: (state: StateTypeTab) => void;
}

export default function CodeSnippetsDesktop({
  activeExample,
  setActiveExample,
  activeStateType,
  setActiveStateType
}: CodeSnippetsDesktopProps) {
  return (
    <div className='flex flex-col gap-2 p-2'>
      {/* Examples Header */}
      <h2 className='text-600 py-0.5 text-center text-sm font-medium text-white/70'>Examples</h2>

      {/* Tabs */}
      <TabGroup
        examples={examples}
        activeExample={activeExample}
        setActiveExample={setActiveExample}
        activeStateType={activeStateType}
        setActiveStateType={setActiveStateType}
      />

      {/* Code Editor with inset rounded rect */}
      <div className='overflow-hidden rounded-xl border border-white/15 bg-[#0d0b0a]'>
        <CodeEditor activeExample={activeExample} activeStateType={activeStateType} />
      </div>

      {/* Bottom bar - outside the inner rect */}
      {/* <BottomBar activeExample={activeExample} /> */}
    </div>
  );
}
