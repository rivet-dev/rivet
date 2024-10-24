import React from 'react';

import { Heading } from '@/components/Heading';
import { Icon } from '@rivet-gg/icons';

import { Button, cn } from '@rivet-gg/components';
import Link from 'next/link';

export function Resource({ children, ...props }) {
  // Adds line break opportunities after periods
  let splitTitle = props.title.split(/\./);
  let titleSegments = React.Children.map(splitTitle, (child, i) => {
    return (
      <div>
        {child}
        {i != splitTitle.length - 1 ? '.' : ''}
        {i != splitTitle.length - 1 ? <wbr></wbr> : null}
      </div>
    );
  });

  return (
    <Button
      variant='outline'
      className='not-prose flex h-auto flex-col items-start pb-4 pt-8 text-left'
      href={props.href}
      {...props}
      startIcon={<Icon icon={props.icon} />}
      asChild>
      <Link>
        <span className='pt-4 font-bold'>{titleSegments}</span>
        {children && (
          <div className='text-muted-foreground mt-2 min-w-0 text-wrap text-sm leading-5'>{children}</div>
        )}
      </Link>
    </Button>
  );
}

export function ResourceGroup({ title = 'Resources', columns = 2, children }) {
  return (
    <div className='my-8'>
      {title && (
        <Heading level={2} id='resources'>
          {title}
        </Heading>
      )}
      <div
        className={cn(
          'not-prose mt-4 grid gap-4',
          'grid-cols-1',
          columns >= 2 && 'sm:grid-cols-2',
          columns == 3 && 'xl:grid-cols-3',
          columns == 4 && 'xl:grid-cols-4'
        )}>
        {children}
      </div>
    </div>
  );
}
