import { Badge, Flex } from "@/components";

interface EnvironmentVersionTitleProps {
	environment: string;
	version: string;
}

export function EnvironmentVersionTitle({
	environment,
	version,
}: EnvironmentVersionTitleProps) {
	return (
		<Flex items="center">
			<span>{environment}</span>
			<Badge ml="4">{version}</Badge>
		</Flex>
	);
}
