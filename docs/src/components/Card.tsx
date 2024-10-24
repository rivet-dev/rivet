import clsx from 'clsx';
import { PropsWithChildren } from 'react';

export function Card({ children, className }: PropsWithChildren<{ className?: string }>) {
  return (
    <div
      className={clsx(
        'flex h-full flex-col items-start gap-1.5 border border-cream-100/5 bg-charcole-900/10 p-6 text-white transition-colors',
        className
      )}>
      {children}
    </div>
  );
}
