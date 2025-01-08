import { ExternalCard, type ExternalCardProps } from "./external-card";

interface DocsCardProps extends Omit<ExternalCardProps, "label"> {}

export const DocsCard = (props: DocsCardProps) => {
	return <ExternalCard {...props} label="Docs" />;
};
