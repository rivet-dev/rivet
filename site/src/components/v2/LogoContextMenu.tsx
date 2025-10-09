"use client";
import { useRef } from "react";
import {
	ContextMenu,
	ContextMenuContent,
	ContextMenuItem,
	ContextMenuTrigger,
	toast,
} from "@rivet-gg/components";
import { Icon, faCopy, faDownload } from "@rivet-gg/icons";

import logoUrl from "@/images/rivet-logos/icon-text-white.svg";

interface LogoContextMenuProps {
	children: React.ReactNode;
}

export function LogoContextMenu({ children }: LogoContextMenuProps) {
	const menuRef = useRef<HTMLDivElement>(null);

	const copyLogoAsSVG = async () => {
		try {
			const response = await fetch(logoUrl.src);
			const svgContent = await response.text();

			await navigator.clipboard.writeText(svgContent);
			toast.success("Logo copied as SVG!");
		} catch (err) {
			toast.error("Failed to copy logo as SVG.");
		}
	};

	const downloadBrandAssets = () => {
		window.open("https://releases.rivet.dev/press-kit.zip", "_blank");
	};

	return (
		<>
			<ContextMenu>
				<ContextMenuTrigger>{children}</ContextMenuTrigger>
				<ContextMenuContent ref={menuRef}>
					<ContextMenuItem onClick={copyLogoAsSVG}>
						<Icon icon={faCopy} className="mr-3 h-4 w-4" />
						Copy logo as SVG
					</ContextMenuItem>
					<ContextMenuItem onClick={downloadBrandAssets}>
						<Icon icon={faDownload} className="mr-3 h-4 w-4" />
						Download brand assets
					</ContextMenuItem>
				</ContextMenuContent>
			</ContextMenu>
		</>
	);
}
