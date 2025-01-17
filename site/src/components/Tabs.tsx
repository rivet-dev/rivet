import {
	Tabs as RivetTabs,
	TabsContent as RivetTabsContent,
	TabsList as RivetTabsList,
	TabsTrigger as RivetTabsTrigger,
	ScrollArea,
} from "@rivet-gg/components";
import { Children } from "react";

export const Tab = ({ title, children }) => {
	return <RivetTabsContent value={title}>{children}</RivetTabsContent>;
};

export const Tabs = ({ children }) => {
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
