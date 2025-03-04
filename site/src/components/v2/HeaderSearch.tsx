"use client";
import { OramaSearchBox } from "@orama/react-components";
import { Button, Dialog, DialogPortal, Kbd, cn } from "@rivet-gg/components";
import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";

export function HeaderSearch() {
	const [isOpen, setIsOpen] = useState(false);
	const router = useRouter();

	useEffect(function setShortcutListener() {
		const handleKeyDown = (e: KeyboardEvent) => {
			if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
				e.preventDefault();
				setIsOpen((prev) => !prev);
			}
		};

		window.addEventListener("keydown", handleKeyDown);
		return () => window.removeEventListener("keydown", handleKeyDown);
	}, []);

	return (
		<>
			<Button
				onClick={() => setIsOpen(true)}
				variant="outline"
				className={cn(
					"relative h-8 w-full justify-start rounded-[0.5rem] bg-background text-sm font-normal text-muted-foreground shadow-none hidden md:flex md:w-24 lg:w-40",
				)}
			>
				<span className="hidden lg:inline-flex">Search...</span>
				<span className="inline-flex lg:hidden">Search...</span>
				<Kbd className="absolute right-[0.3rem] top-1/2 -translate-y-1/2 hidden sm:flex">
					<Kbd.Key />K
				</Kbd>
			</Button>
			<Dialog open={isOpen}>
				<DialogPortal>
					<OramaSearchBox
						open={isOpen}
						layout="modal"
						onModalClosed={() => setIsOpen(false)}
						colorScheme="system"
						onSearchResultClick={(event) => {
							event.preventDefault();
							router.push(event.detail.result.path);
						}}
						searchPlaceholder="Search something..."
						index={{
							endpoint:
								"https://cloud.orama.run/v1/indexes/rivet-gg-b87fiw",
							api_key: "dcVm1fAKZeTdOfGFZCCH9xWiH7JeYCQZ",
						}}
						resultMap={{
							title: "name",
							description: "content",
							section: "category",
						}}
					/>
				</DialogPortal>
			</Dialog>
		</>
	);
}
