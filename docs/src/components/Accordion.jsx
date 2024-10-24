'use client';
import { Icon, faCaretDown, faCaretRight } from '@rivet-gg/icons';
import { clsx } from 'clsx';
import React, { useState } from 'react';

function getAccordionStyleFromVariant(variant) {
  if (variant === 'minimalist') {
    // Minimal padding. Shows a border along the left when open.
    return {
      parentClass: '',
      coverClass:
        '[&>div]:ml-2 py-1 text-charcole-700 hover:text-charcole-900 dark:text-cream-400 dark:hover:text-cream-200',
      contentClass: 'mt-2 pt-1 mb-4 mx-[7px] px-4 border-l border-cream-100 dark:border-charcole-800'
    };
  }

  // Rounding is handled in Accordion by passing in isRounded to AccordionCover.
  return {
    parentClass: 'border dark:border-charcole-800 rounded-xl mb-3 overflow-hidden',
    coverClass: 'py-4 px-5 space-x-2 hover:bg-violet-100 hover:dark:bg-white/10 rounded-t-xl transition',
    contentClass: 'mt-2 mb-4 mx-6'
  };
}

export function Accordion({
  title,
  description,
  defaultOpen = false,
  icon,
  onChange,
  variant = 'rounded',
  children
}) {
  const [open, setOpen] = useState(defaultOpen);

  const onClickOpen = open => {
    setOpen(open);
    if (onChange) {
      onChange(open);
    }
  };

  const { parentClass, coverClass, contentClass } = getAccordionStyleFromVariant(variant);

  return (
    <div key={title} role='listitem' className={parentClass}>
      <AccordionCover
        title={title}
        description={description}
        open={open}
        setOpen={onClickOpen}
        icon={icon}
        coverClass={coverClass}
      />
      <div className={clsx(contentClass, !open && 'hidden')}>{children}</div>
    </div>
  );
}

export function AccordionGroup({ children }) {
  // [&>div] modifies the Accordion's borders to only show divider borders.
  // We use border-0 instead of border-none because border-none turns off divide-y.
  // [&>div>button] modifies the button to not round the highlighted part
  // when inside of an Accordion group.
  return (
    <div
      className='mb-3 mt-0 divide-y divide-inherit overflow-hidden rounded-xl border dark:border-charcole-800 [&>div>button]:rounded-none [&>div]:mb-0 [&>div]:rounded-none [&>div]:border-0'
      role='list'>
      {children}
    </div>
  );
}

function AccordionCover({ title, description, open, setOpen, icon, coverClass }) {
  // In rounded style, we round the button itself so when a web browser in keyboard navigation mode
  // highlights the button the highlight will follow the corners.
  return (
    <button
      onClick={() => setOpen(!open)}
      className={clsx('not-prose flex w-full flex-row content-center items-center', coverClass)}
      aria-controls={title + 'Children'}
      aria-expanded={open}>
      <div className='mr-0.5'>
        <Icon
          icon={open ? faCaretDown : faCaretRight}
          className='h-3 w-3 text-charcole-800 opacity-75 dark:text-cream-100'
        />
      </div>
      {icon ? (
        <div className='h-4 w-4 fill-charcole-800 text-charcole-800 dark:fill-cream-100 dark:text-cream-100'>
          {icon}
        </div>
      ) : null}
      <div className='text-left leading-tight'>
        <p className='m-0 font-medium text-charcole-900 dark:text-cream-200'>{title}</p>
        {description ? <p className='m-0 text-charcole-900 dark:text-cream-200'>{description}</p> : null}
      </div>
    </button>
  );
}
