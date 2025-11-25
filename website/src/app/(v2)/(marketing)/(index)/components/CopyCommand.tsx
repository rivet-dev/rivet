'use client';

import { Icon, faArrowRight, faCheck, faCopy } from '@rivet-gg/icons';
import clsx from 'clsx';
import { useState } from 'react';

interface CopyCommandProps {
  children?: string;
  command?: string;
  className?: string;
}

export function CopyCommand({ children, command, className }: CopyCommandProps) {
  const [copied, setCopied] = useState(false);

  const handleCopy = () => {
    const commandText = command || children || '';
    const textToCopy = commandText.startsWith('$') ? commandText.substring(1).trim() : commandText;

    navigator.clipboard.writeText(textToCopy);

    setCopied(true);
    setTimeout(() => {
      setCopied(false);
    }, 1000);
  };

  return (
    <div
      className={clsx(
        'group inline-flex max-w-fit cursor-pointer items-center gap-2 rounded-lg px-3 py-2 transition-all duration-200',
        className
      )}
      onClick={handleCopy}
    >
      <div className='flex h-6 w-6 items-center justify-center text-white/40 transition-colors duration-200 group-hover:text-white/80'>
        <Icon icon={faArrowRight} className='h-4 w-4' />
      </div>

      <div className='font-mono text-sm font-medium text-white/40 transition-colors duration-200 group-hover:text-white/80'>
        {command || children}
      </div>

      <div className='relative flex h-5 w-5 items-center justify-center'>
        <div
          className={`absolute transition-opacity duration-200 ${
            copied ? 'opacity-0' : 'opacity-0 group-hover:opacity-100'
          }`}
        >
          <Icon icon={faCopy} className='h-4 w-4 text-white/40 transition-colors group-hover:text-white/80' />
        </div>
        <div className={`absolute transition-opacity duration-200 ${copied ? 'opacity-100' : 'opacity-0'}`}>
          <Icon icon={faCheck} className='h-4 w-4 text-green-400' />
        </div>
      </div>
    </div>
  );
}
