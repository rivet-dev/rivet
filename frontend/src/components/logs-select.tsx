import type { ReactNode } from "react";
import { Flex } from "./ui/flex";

interface LogsSelectProps {
	list: ReactNode;
	content: ReactNode;
}

export function LogsSelect({ list, content }: LogsSelectProps) {
	return (
		<div className="flex flex-col lg:grid lg:grid-cols-[minmax(0,1fr),minmax(0,1fr)] gap-4">
			<Flex direction="col" gap="2">
				{list}
			</Flex>
			<div className="border-t pt-4 lg:border-t-0 lg:pt-0 lg:border-l lg:pl-4 w-full">
				{content}
			</div>
		</div>
	);
}
