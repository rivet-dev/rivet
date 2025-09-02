import { faEllipsisH, Icon } from "@rivet-gg/icons";
import { useNavigate } from "@tanstack/react-router";
import {
	Button,
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
} from "@/components";

interface ProjectBuildsTableActionsProps {
	buildId: string;
}

export function ProjectBuildsTableActions({
	buildId,
}: ProjectBuildsTableActionsProps) {
	const navigate = useNavigate();
	return (
		<DropdownMenu>
			<DropdownMenuTrigger asChild>
				<Button aria-haspopup="true" size="icon" variant="ghost">
					<Icon className="size-4" icon={faEllipsisH} />
					<span className="sr-only">Toggle menu</span>
				</Button>
			</DropdownMenuTrigger>
			<DropdownMenuContent align="end">
				<DropdownMenuItem
					onSelect={() => {
						navigate({
							to: ".",
							search: { modal: "edit-tags", buildId },
						});
					}}
				>
					Edit tags
				</DropdownMenuItem>
			</DropdownMenuContent>
		</DropdownMenu>
	);
}
