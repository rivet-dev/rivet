import { cn } from '@rivet-gg/components';

export function Steps({ className, children }) {
  return (
    <div className={cn('[&>h3]:step steps mb-12 ml-4 border-l pl-8 [counter-reset:step]', className)}>
      {children}
    </div>
  );
}

export function Step({ children, className, title }) {
  return (
    <>
      <h3 className={cn('font-heading mt-8 scroll-m-20 text-xl font-semibold tracking-tight', className)}>
        {title}
      </h3>
      {children}
    </>
  );
}
