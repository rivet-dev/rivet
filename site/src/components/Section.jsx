import { useId } from 'react';

export function Section({ title, children }) {
  let id = useId();

  return (
    <section
      aria-labelledby={id}
      className='md:border-l md:border-cream-100 md:pl-6 md:dark:border-charcole-700/40'
    >
      <div className='grid max-w-3xl grid-cols-1 items-baseline gap-y-8 md:grid-cols-4'>
        <h2 id={id} className='text-sm font-semibold text-charcole-800 dark:text-cream-100'>
          {title}
        </h2>
        <div className='md:col-span-3'>{children}</div>
      </div>
    </section>
  );
}
