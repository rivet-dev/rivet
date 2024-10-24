'use client';
import Link from 'next/link';
import { Button } from '@rivet-gg/components';

import { Tag } from '@/components/Tag';
import { Icon, faLink } from '@rivet-gg/icons';

function Eyebrow({ tag, label }) {
  if (!tag && !label) {
    return null;
  }

  return (
    <div className='flex items-center gap-x-3'>
      {tag && <Tag>{tag}</Tag>}
      {tag && label && <span className='h-0.5 w-0.5 rounded-full bg-cream-300 dark:bg-charcole-600' />}
      {label && <span className='font-mono text-xs text-cream-400'>{label}</span>}
    </div>
  );
}

function Anchor({ id, children }) {
  return (
    <Button
      className='not-prose absolute -left-10 top-1 hidden group-hover:flex'
      size='icon-sm'
      variant='outline'
      asChild>
      <Link href={`#${id}`}>
        <Icon icon={faLink} />
      </Link>
    </Button>
  );
}

export function Heading({ level = 2, children, id, tag, label, anchor = true, ...props }) {
  let Component = `h${level}`;

  return (
    <>
      {level == 2 && <hr />}
      <Eyebrow tag={tag} label={label} />
      <Component
        id={anchor ? id : undefined}
        className={
          tag || label ? 'mt-2 scroll-mt-32' : 'scroll-mt-header group relative scroll-mt-header-offset'
        }
        {...props}>
        {anchor ? <Anchor id={id} /> : null}
        {anchor ? (
          <Link className='group text-inherit no-underline hover:text-inherit' href={`#${id}`}>
            {children}
          </Link>
        ) : (
          children
        )}
      </Component>
    </>
  );
}
