import { Link } from "react-router-dom";
import { HomeIcon } from "lucide-react";

interface TopBarProps {
	appName: string;
}

export function TopBar({ appName }: TopBarProps) {
	return (
		<div className="h-12 sticky top-0 flex items-center px-4 border-b bg-background justify-between">
			<div className="flex items-center gap-3">
				<Link to="/" className="p-2 hover:bg-accent rounded-md">
					<HomeIcon className="h-4 w-4" />
				</Link>
				<h1 className="font-medium truncate">{appName}</h1>
			</div>
		</div>
	);
}
