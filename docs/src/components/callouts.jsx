function InfoIcon(props) {
  return (
    <svg viewBox='0 0 16 16' aria-hidden='true' {...props}>
      <circle cx='8' cy='8' r='8' strokeWidth='0' />
      <path
        fill='none'
        strokeLinecap='round'
        strokeLinejoin='round'
        strokeWidth='1.5'
        d='M6.75 7.75h1.5v3.5'
      />
      <circle cx='8' cy='4' r='.5' fill='none' />
    </svg>
  );
}

export function Tip({ children }) {
  return (
    <div className='my-6 flex gap-2.5 border border-emerald-500/30 bg-emerald-500/5 p-4 leading-6 text-emerald-200 [--tw-prose-links-hover:theme(colors.emerald.300)] [--tw-prose-links:theme(colors.white)]'>
      <InfoIcon className='mt-1 h-4 w-4 flex-none fill-emerald-200/20 stroke-emerald-200' />
      <div className='[&>:first-child]:mt-0 [&>:last-child]:mb-0'>{children}</div>
    </div>
  );
}

export function Note({ children }) {
  return (
    <div className='my-6 flex gap-2.5 border border-violet-500/30 bg-violet-500/5 p-4 leading-6 text-violet-200 [--tw-prose-links:theme(colors.white)] [--tw-prose-links-hover:theme(colors.violet.300)]'>
      <InfoIcon className='mt-1 h-4 w-4 flex-none fill-violet-200/20 stroke-violet-200' />
      <div className='[&>:first-child]:mt-0 [&>:last-child]:mb-0'>{children}</div>
    </div>
  );
}

export function Info({ children }) {
  return (
    <div className='my-6 flex gap-2.5 border  border-cyan-500/30 bg-cyan-500/5  p-4 leading-6 text-cyan-200 [--tw-prose-links:theme(colors.white)] [--tw-prose-links-hover:theme(colors.cyan.300)]'>
      <InfoIcon className='mt-1 h-4 w-4 flex-none  fill-cyan-200/20 stroke-cyan-200' />
      <div className='[&>:first-child]:mt-0 [&>:last-child]:mb-0'>{children}</div>
    </div>
  );
}

export function Warning({ children }) {
  return (
    <div className='my-6 flex gap-2.5 border  border-amber-500/30 bg-amber-500/5  p-4 leading-6 text-amber-200 [--tw-prose-links:theme(colors.white)] [--tw-prose-links-hover:theme(colors.amber.300)]'>
      <InfoIcon className='mt-1 h-4 w-4 flex-none  fill-amber-200/20 stroke-amber-200' />
      <div className='[&>:first-child]:mt-0 [&>:last-child]:mb-0'>{children}</div>
    </div>
  );
}
