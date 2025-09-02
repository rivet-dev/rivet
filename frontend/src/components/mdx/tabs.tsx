import { Children } from "react";
import { ScrollArea } from "../ui/scroll-area";
import {
	Tabs as RivetTabs,
	TabsContent as RivetTabsContent,
	TabsList as RivetTabsList,
	TabsTrigger as RivetTabsTrigger,
} from "../ui/tabs";

export const Tab = ({
	title,
	children,
}: {
	title: string;
	children: React.ReactNode;
}) => {
	return <RivetTabsContent value={title}>{children}</RivetTabsContent>;
};

export const Tabs = ({ children }: { children: React.ReactElement }) => {
	const titles = Children.map(children, (child) => child.props.title);
	return (
		<RivetTabs defaultValue={titles[0]}>
			<ScrollArea>
				<RivetTabsList>
					{titles.map((title) => (
						<RivetTabsTrigger key={title} value={title}>
							{title}
						</RivetTabsTrigger>
					))}
				</RivetTabsList>
			</ScrollArea>
			{children}
		</RivetTabs>
	);
};
