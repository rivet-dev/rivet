import { type IconProp, library } from "@fortawesome/fontawesome-svg-core";
import { Icon, iconPack } from "@rivet-gg/icons";

// @ts-ignore
library.add(iconPack);

interface ModuleIconProps {
	className?: string;
	icon: IconProp;
}

export function ModuleIcon({ className, icon }: ModuleIconProps) {
	return <Icon icon={icon} className={className} />;
}
