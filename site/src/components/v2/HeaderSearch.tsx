"use client";
import { OramaSearchBox } from "@orama/react-components";
import { Button, Dialog, DialogPortal, Kbd, cn } from "@rivet-gg/components";
import { useState } from "react";

export function HeaderSearch() {
	const [isOpen, setIsOpen] = useState(false);
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
				<Kbd className="absolute right-[0.3rem] top-[0.3rem] hidden sm:flex">
					<Kbd.Cmd />K
				</Kbd>
			</Button>
			<Dialog open={isOpen}>
				<DialogPortal>
					<OramaSearchBox
						open={isOpen}
						layout="modal"
						onModalStatusChanged={(status) => setIsOpen(!status)}
						colorScheme="system"
						onSearchResultClick={() => setIsOpen(false)}
						placeholder="Search something..."
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
