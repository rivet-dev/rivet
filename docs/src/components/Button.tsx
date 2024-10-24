import Link from 'next/link';
import clsx from 'clsx';
import { Icon } from '@rivet-gg/icons';

function ArrowIcon(props) {
  return (
    <svg viewBox='0 0 20 20' fill='none' aria-hidden='true' {...props}>
      <path
        stroke='currentColor'
        strokeLinecap='round'
        strokeLinejoin='round'
        d='m11.5 6.5 3 3.5m0 0-3 3.5m3-3.5h-9'
      />
    </svg>
  );
}

const commonAnimationClasses = [
  'transition-[background,transform,border-color,color,box-shadow] duration-200 ease-out',
  'hover:-translate-y-[2px] hover:shadow-[0_4px_10px_0_rgba(0,0,0,0.6)]',
  'active:opacity-75'
];

const variantClasses = {
  primary: {
    base: clsx(
      // Base styles
      'inline-flex items-center justify-center gap-0.5',
      'rounded-sm px-3 py-1',
      'text-sm font-semibold',
      'bg-cream-100 text-charcole-950',
      // Hover
      'hover:bg-cream-50 hover:text-charcole-950'
    ),
    highlight: ''
  },
  secondary: {
    base: clsx(
      // Base styles
      'inline-flex items-center justify-center gap-0.5',
      'rounded-sm px-3 py-1',
      'text-sm font-semibold',
      'bg-transparent text-cream-100',
      'border-2 border-cream-100',
      // Hover
      'hover:bg-cream-100 hover:text-charcole-950'
    ),
    highlight: ''
  },
  text: {
    base: clsx(
      'text-sm text-orange-300',
      // Hover
      'hover:text-orange-500'
    ),
    highlight: ''
  },
  'text-subtle': {
    base: clsx(
      'text-sm text-charcole-400',
      // Hover
      'hover:text-charcole-300'
    ),
    highlight: ''
  },
  juicy: {
    base: clsx([
      // Base styles
      'inline-flex items-center justify-center gap-2',
      'rounded px-4 py-2',
      'text-sm font-bold',
      'bg-charcole-900/30 text-cream-100',
      'border-2 border-cream-100/5',
      ...commonAnimationClasses,
      // Hover
      'hover:border-cream-100/20 hover:bg-charcole-800/50 hover:text-cream-50',
      // Selected
      'aria-selected:border-cream-100/30 aria-selected:text-cream-50',
      // Disabled
      'disabled:border-cream-100 disabled:opacity-60 disabled:hover:bg-transparent disabled:hover:text-cream-100',
      // Loading
      'aria-busy:translate-y-0 aria-busy:border-neutral-300 aria-busy:hover:bg-transparent aria-busy:hover:text-white'
    ]),
    normal: 'text-cream-100',
    highlight: 'text-charcole-950'
  },
  juicySubtle: {
    base: clsx([
      // Base styles
      'inline-flex items-center justify-center gap-2',
      'rounded px-4 py-2',
      'text-sm font-bold',
      'bg-transparent text-cream-100',
      'border-2 border-cream-100/10',
      ...commonAnimationClasses,
      'relative',
      "before:absolute before:inset-0 before:-z-10 before:opacity-0 before:transition-all before:content-['']",
      "after:absolute after:inset-0 after:-z-10 after:opacity-100 after:transition-all after:content-['']",
      // Hover
      'hover:border-cream-100/20 hover:bg-cream-100/10',
      // Selected
      'aria-selected:border-cream-100/30 aria-selected:text-cream-100 aria-selected:before:border-cream-100/30',
      // Disabled
      'disabled:border-cream-100 disabled:opacity-60 disabled:hover:bg-transparent disabled:hover:text-cream-100',
      'disabled:hover:before:opacity-0 disabled:hover:after:opacity-100',
      // Loading
      'aria-busy:translate-y-0 aria-busy:border-neutral-300 aria-busy:hover:bg-transparent aria-busy:hover:text-white'
    ]),
    normal: 'text-cream-100',
    highlight: 'text-charcole-950'
  },
  primaryJuicy: {
    base: clsx([
      // Base styles
      'inline-flex items-center justify-center gap-2',
      'rounded px-4 py-2',
      'text-sm font-bold',
      'bg-cream-100 text-charcole-950',
      'border-2 border-charcole-950/15',
      ...commonAnimationClasses,
      'relative',
      'before:bg-cream-100 after:bg-cream-100',
      'before:opacity-100 after:opacity-0',
      // Hover
      'hover:bg-orange-400 hover:text-charcole-950',
      'hover:before:bg-cream-100 hover:before:opacity-100 hover:after:opacity-0',
      // Selected
      'aria-selected:bg-cream-100 aria-selected:text-charcole-950 aria-selected:before:bg-cream-100',
      'aria-selected:before:opacity-100 aria-selected:after:opacity-0',
      // Active
      'aria-selected:bg-cream-50',
      // Disabled
      'disabled:opacity-60 disabled:hover:bg-transparent disabled:hover:text-cream-100',
      'disabled:hover:before:opacity-0 disabled:hover:after:opacity-100',
      // Loading
      'aria-busy:translate-y-0 aria-busy:hover:bg-transparent aria-busy:hover:text-white'
    ]),
    normal: 'text-cream-100',
    highlight: 'text-charcole-950'
  },
  blackJuicy: {
    base: clsx(
      // Base styles
      'inline-flex items-center justify-center gap-0.5',
      'rounded-sm px-3.5 py-2.5',
      'text-sm font-semibold',
      'bg-transparent text-black',
      'border-2 border-black',
      // Hover
      'hover:bg-black hover:text-cream-100'
    ),
    highlight: ''
  }
};

export function ButtonGroup({ children }) {
  return <div className='not-prose mb-16 mt-6 flex gap-3'>{children}</div>;
}

interface CommonButtonProps {
  variant?: keyof typeof variantClasses;
  highlight?: boolean;
  styles?: React.CSSProperties;
  className?: string;
  children?: React.ReactNode;
  arrow?: 'left' | 'right';
  icon?: any;
}

interface AnchorButtonProps extends CommonButtonProps {
  href: string;
  target?: string;
  rel?: string;
}

interface ButtonButtonProps extends CommonButtonProps {
  href: never;
}

type ButtonProps = AnchorButtonProps | ButtonButtonProps;

export function Button({
  variant = 'primary',
  highlight = false,
  styles,
  className,
  children,
  arrow,
  icon,
  ...props
}: ButtonProps) {
  let Component = 'href' in props && props.href ? Link : 'button';
  className = clsx(
    'relative inline-flex items-center justify-center gap-0.5 overflow-hidden font-semibold transition',
    variantClasses[variant].base,
    className
  );

  let arrowIcon = (
    <ArrowIcon
      className={clsx(
        'h-5 w-5',
        variant === 'text' && 'relative top-px',
        arrow === 'left' && '-ml-1 rotate-180',
        arrow === 'right' && '-mr-1'
      )}
    />
  );

  return (
    <Component aria-selected={highlight} className={className} {...props}>
      {icon ? <Icon icon={icon} className='-ml-1 h-5 w-5' /> : null}
      {arrow === 'left' && arrowIcon}
      {children}
      {arrow === 'right' && arrowIcon}
    </Component>
  );
}
