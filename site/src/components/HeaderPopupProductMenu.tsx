import { faBriefcase } from '@fortawesome/free-solid-svg-icons/faBriefcase';
import { faCloud } from '@fortawesome/free-solid-svg-icons/faCloud';
import { faCodeBranch } from '@fortawesome/free-solid-svg-icons/faCodeBranch';
import { cn, Button } from '@rivet-gg/components';
import { faActors, Icon } from '@rivet-gg/icons';
import { ComponentProps, ReactNode } from 'react';

import Link from 'next/link';

export const HeaderPopupProductMenu = () => {
  return (
    <div className='grid h-full grid-cols-3 grid-rows-3 gap-4 overflow-hidden pb-2'>
      <Link href='/docs' className='col-span-2 row-span-3 '>
        <Item
          onMouseEnter={e => e.currentTarget.querySelector('video')?.play()}
          onMouseLeave={e => e.currentTarget.querySelector('video')?.pause()}>
          <div className='relative z-10 h-full'>
            <p className='text-base font-bold opacity-80 transition-opacity group-hover:opacity-100'>
              <Icon icon={faActors} className='mr-1' />
              Actors
            </p>
            <p className='opacity-80 transition-opacity group-hover:opacity-100'>
              The easiest way to build & scale realtime applications.
            </p>
          </div>
          <video
            className='absolute inset-0 h-full w-full object-cover opacity-60'
            muted
            loop
            playsInline
            disablePictureInPicture
            disableRemotePlayback>
            <source
              src='https://assets2.rivet.gg/effects/bg-effect-product-actors.webm?v=2'
              type='video/webm'
            />
          </video>
        </Item>
      </Link>

      <Button
        variant='secondary'
        asChild
        className='col-start-3 h-full justify-start'
        startIcon={<Icon icon={faCodeBranch} />}>
        <Link href='https://github.com/rivet-gg/rivet' target='_blank'>
          Community Edition
        </Link>
      </Button>
      <Button
        variant='secondary'
        className='col-start-3 h-full justify-start'
        startIcon={<Icon icon={faCloud} />}>
        <Link href='https://hub.rivet.gg' target='_blank'>
          Rivet Cloud
        </Link>
      </Button>
      <Button
        variant='secondary'
        className='col-start-3 h-full justify-start'
        target='_blank'
        startIcon={<Icon icon={faBriefcase} />}>
        <Link href='/sales'>Rivet Enterprise</Link>
      </Button>
    </div>
  );
};

interface ItemProps extends ComponentProps<'div'> {
  className?: string;
  children?: ReactNode;
}
function Item({ className, children, ...props }: ItemProps) {
  return (
    <div
      className={cn(
        'group h-full cursor-pointer overflow-hidden rounded-md p-4 text-sm grayscale transition-all hover:grayscale-0',
        className
      )}
      {...props}>
      {children}
    </div>
  );
}
