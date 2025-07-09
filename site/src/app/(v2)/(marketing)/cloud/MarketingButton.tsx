import Link from "next/link";

// Marketing Button component for consistent styling across marketing pages
export const MarketingButton = ({ 
  children, 
  href, 
  primary = false 
}: { 
  children: React.ReactNode; 
  href: string; 
  primary?: boolean;
}) => {
  return (
    <Link 
      href={href}
      className={`group inline-flex items-center justify-center px-3.5 py-2 text-base font-medium rounded-xl transition-all duration-200 active:scale-[0.97] ${
        primary 
          ? "bg-[#FF5C00]/90 hover:bg-[#FF5C00] hover:brightness-110 text-white"
          : "text-white/80 hover:text-white bg-transparent hover:bg-[rgba(255,255,255,0.1)]"
      }`}
    >
      {children}
    </Link>
  );
};
