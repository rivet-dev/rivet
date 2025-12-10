import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Share2Icon, CheckIcon } from "lucide-react";
import { toast } from "sonner";

interface ShareButtonProps {
	className?: string;
	devServerUrl?: string;
}

export function ShareButton({ className, devServerUrl }: ShareButtonProps) {
	const [copied, setCopied] = useState(false);

	const handleCopy = () => {
		if (!devServerUrl) {
			toast.error("Dev server URL not available yet");
			return;
		}

		navigator.clipboard
			.writeText(devServerUrl)
			.then(() => {
				setCopied(true);
				toast.success("Link copied to clipboard!");
				setTimeout(() => setCopied(false), 2000);
			})
			.catch(() => {
				toast.error("Failed to copy link");
			});
	};

	return (
		<Button
			variant="ghost"
			size="sm"
			className={`gap-1.5 cursor-pointer ${className || ""}`}
			onClick={handleCopy}
			disabled={!devServerUrl}
		>
			{copied ? (
				<CheckIcon className="h-4 w-4" />
			) : (
				<Share2Icon className="h-4 w-4" />
			)}
			{copied ? "Copied!" : "Share"}
		</Button>
	);
}
