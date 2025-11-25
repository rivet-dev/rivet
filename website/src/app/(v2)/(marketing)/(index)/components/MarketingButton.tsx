import Link from 'next/link';

// Marketing Button component for consistent styling across marketing pages
export const MarketingButton = ({
  children,
  href,
  target,
  rel,
  primary = false
}: {
  children: React.ReactNode;
  href: string;
  target?: string;
  rel?: string;
  primary?: boolean;
}) => {
  return (
    <Link
      href={href}
      target={target}
      rel={rel}
      className={`group inline-flex h-11 items-center justify-center rounded-xl px-4 py-2.5 text-base font-medium transition-all duration-200 active:scale-[0.97] ${
        primary
          ? 'bg-[#FF5C00]/90 text-white hover:bg-[#FF5C00] hover:brightness-110'
          : 'border border-white/20 bg-transparent text-white/80 hover:border-white/40 hover:bg-[rgba(255,255,255,0.1)] hover:text-white'
      }`}
    >
      {children}
    </Link>
  );
};
