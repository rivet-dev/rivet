import Link from 'next/link';

import { Icon } from '@/components/Icon';

export function CardGroup({ children }) {
  return <div className='not-prose my-12 grid grid-cols-1 gap-6 sm:grid-cols-2'>{children}</div>;
}

export function Card({ title, description, href, icon }) {
  return (
    <div className='group relative rounded-xl border border-cream-200 dark:border-charcole-800'>
      <div className='absolute -inset-px rounded-xl border-2 border-transparent opacity-0 [background:linear-gradient(var(--quick-links-hover-bg,theme(colors.sky.50)),var(--quick-links-hover-bg,theme(colors.sky.50)))_padding-box,linear-gradient(to_top,theme(colors.violet.400),theme(colors.cyan.400),theme(colors.sky.500))_border-box] group-hover:opacity-100 dark:[--quick-links-hover-bg:theme(colors.slate.800)]' />
      <div className='relative overflow-hidden rounded-xl p-6'>
        <Icon icon={icon} className='h-8 w-8' />
        <h2 className='mt-4 font-display text-base text-charcole-900 dark:text-white'>
          <Link href={href}>
            <span className='absolute -inset-px rounded-xl' />
            {title}
          </Link>
        </h2>
        <p className='mt-1 text-sm text-charcole-700 dark:text-cream-400'>{description}</p>
      </div>
    </div>
  );
}
