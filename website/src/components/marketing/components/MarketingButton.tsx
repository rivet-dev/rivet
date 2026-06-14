// Marketing Button component for consistent styling across marketing pages.
//
// Variants on the porcelain system:
// - ember: the spark. At most one per page (hero CTA or a form's submit).
// - primary: the default call to action, ink block with cream text.
// - secondary: quiet hairline outline.
// - ghost: pine text link with no chrome.
type MarketingButtonVariant = 'ember' | 'primary' | 'secondary' | 'ghost';

const VARIANT_CLASSES: Record<MarketingButtonVariant, string> = {
	ember: 'bg-accent-deep text-white hover:bg-accent',
	primary: 'bg-ink text-cream hover:bg-ink/85',
	secondary: 'border border-ink/20 bg-transparent text-ink hover:border-ink/40 hover:bg-ink/[0.04]',
	ghost: 'px-0 text-pine hover:underline underline-offset-4',
};

export const MarketingButton = ({
	children,
	href,
	target,
	rel,
	variant,
	primary = false,
}: {
	children: React.ReactNode;
	href: string;
	target?: string;
	rel?: string;
	variant?: MarketingButtonVariant;
	// Deprecated boolean kept while pages migrate to `variant`.
	primary?: boolean;
}) => {
	const resolved: MarketingButtonVariant = variant ?? (primary ? 'ember' : 'secondary');
	return (
		<a
			href={href}
			target={target}
			rel={rel}
			className={`group inline-flex h-11 items-center justify-center rounded-xl px-5 py-2.5 text-base font-medium transition-all duration-200 active:scale-[0.97] ${VARIANT_CLASSES[resolved]}`}
		>
			{children}
		</a>
	);
};
