import clsx from 'clsx';

import { Icon } from '@/components/Icon';

const styles = {
  note: {
    container: 'bg-sky-50 dark:bg-charcole-950/60 dark:ring-1 dark:ring-cream-300/10',
    title: 'text-sky-900 dark:text-sky-400',
    body: 'text-sky-800 [--tw-prose-background:theme(colors.sky.50)] prose-a:text-sky-900 prose-code:text-sky-900 dark:text-cream-100 dark:prose-code:text-cream-100'
  },
  warning: {
    container: 'bg-amber-50 dark:bg-charcole-950/60 dark:ring-1 dark:ring-cream-300/10',
    title: 'text-amber-900 dark:text-amber-500',
    body: 'text-amber-800 [--tw-prose-underline:theme(colors.amber.400)] [--tw-prose-background:theme(colors.amber.50)] prose-a:text-amber-900 prose-code:text-amber-900 dark:text-cream-100 dark:[--tw-prose-underline:theme(colors.sky.700)] dark:prose-code:text-cream-100'
  }
};

const icons = {
  note: props => <Icon icon='lightbulb' {...props} />,
  warning: props => <Icon icon='warning' color='amber' {...props} />
};

export function Callout({ type = 'note', title, children }) {
  let IconComponent = icons[type];

  return (
    <div className={clsx('my-8 flex p-6', styles[type].container)}>
      <IconComponent className='h-8 w-8 flex-none' />
      <div className='ml-4 flex-auto'>
        <p className={clsx('m-0 font-display text-xl', styles[type].title)}>{title}</p>
        <div className={clsx('prose mt-2.5', styles[type].body)}>{children}</div>
      </div>
    </div>
  );
}
