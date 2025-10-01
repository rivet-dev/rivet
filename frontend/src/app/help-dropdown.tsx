import {
	faBooks,
	faComments,
	faDiscord,
	faGithub,
	Icon,
} from "@rivet-gg/icons";
import type { ReactNode } from "react";
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
} from "@/components";

export const HelpDropdown = ({ children }: { children: ReactNode }) => {
	return (
		<DropdownMenu>
			<DropdownMenuTrigger asChild>{children}</DropdownMenuTrigger>
			<DropdownMenuContent>
				<DropdownMenuItem
					indicator={<Icon icon={faGithub} />}
					onSelect={() => {
						window.open(
							"https://github.com/rivet-dev/engine/issues",
							"_blank",
						);
					}}
				>
					GitHub
				</DropdownMenuItem>
				<DropdownMenuItem
					indicator={<Icon icon={faDiscord} />}
					onSelect={() => {
						window.open("https://rivet.dev/discord", "_blank");
					}}
				>
					Discord
				</DropdownMenuItem>
				<DropdownMenuItem
					indicator={<Icon icon={faBooks} />}
					onSelect={() => {
						window.open("https://rivet.dev/docs", "_blank");
					}}
				>
					Documentation
				</DropdownMenuItem>
				{__APP_TYPE__ === "cloud" ? (
					<DropdownMenuItem
						indicator={<Icon icon={faComments} />}
						onSelect={() => {
							Plain.open();
						}}
					>
						Live Chat
					</DropdownMenuItem>
				) : null}
			</DropdownMenuContent>
		</DropdownMenu>
	);
};
