import {
	faBooks,
	faComments,
	faDiscord,
	faGithub,
	faMessageSmile,
	Icon,
} from "@rivet-gg/icons";
import { useNavigate } from "@tanstack/react-router";
import type { ReactNode } from "react";
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
} from "@/components";

export const HelpDropdown = ({ children }: { children: ReactNode }) => {
	const navigate = useNavigate();
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
				<DropdownMenuItem
					indicator={<Icon icon={faMessageSmile} />}
					onSelect={() => {
						navigate({
							to: ".",
							search: (old) => ({ ...old, modal: "feedback" }),
						});
					}}
				>
					Feedback
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
