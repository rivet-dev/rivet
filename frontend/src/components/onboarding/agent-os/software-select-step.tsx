import { useController } from "react-hook-form";
import { DEFAULT_PACKAGES, SOFTWARE } from "./catalog";
import { SelectCard } from "./select-card";

// Presentational multi-select for the software packages baked into the build.
export function SoftwareSelect({
	value,
	onChange,
}: {
	value: string[];
	onChange: (slugs: string[]) => void;
}) {
	const toggle = (slug: string) => {
		onChange(
			value.includes(slug)
				? value.filter((s) => s !== slug)
				: [...value, slug],
		);
	};

	return (
		<div className="flex flex-col gap-3">
			<p className="text-xs text-muted-foreground">
				Packages are baked into the build image and are immutable after
				deploy, so choose what your agent needs now.
			</p>
			<div className="grid grid-cols-2 gap-2">
				{SOFTWARE.map((pkg) => (
					<SelectCard
						key={pkg.slug}
						multi
						title={pkg.title}
						description={pkg.description}
						selected={value.includes(pkg.slug)}
						onSelect={() => toggle(pkg.slug)}
					/>
				))}
			</div>
		</div>
	);
}

// Wizard step: binds SoftwareSelect to the react-hook-form `packages` field.
export function SoftwareSelectStep() {
	const { field } = useController({ name: "packages" });
	return (
		<SoftwareSelect
			value={(field.value as string[]) ?? DEFAULT_PACKAGES}
			onChange={field.onChange}
		/>
	);
}
