import {
	Icon,
	faBluesky,
	faDiscord,
	faGithub,
	faXTwitter,
} from "@rivet-gg/icons";
import Link from "next/link";

export function CommunitySection() {
	const communityLinks = [
		{
			href: "https://rivet.dev/discord",
			icon: faDiscord,
			label: "Discord",
		},
		{
			href: "https://x.com/rivet_dev",
			icon: faXTwitter,
			label: "X",
		},
		{
			href: "https://bsky.app/profile/rivet.dev",
			icon: faBluesky,
			label: "Bluesky",
		},
		{
			href: "https://github.com/rivet-dev/rivetkit/discussions",
			icon: faGithub,
			label: "Discussions",
		},
		{
			href: "https://github.com/rivet-dev/rivetkit/issues",
			icon: faGithub,
			label: "Issues",
		},
	];

	return (
		<div className="text-center mx-auto max-w-6xl">
			<div className="mb-16">
				<h2 className="text-2xl sm:text-3xl font-700 text-white mb-6">
					Join the Community
				</h2>
				<p className="text-lg font-500 text-white/40 max-w-2xl mx-auto">
					Join thousands of developers building with Rivet Actors
					today
				</p>
			</div>

			<div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-5 gap-4 max-w-4xl mx-auto">
				{communityLinks.map((link, index) => (
					<Link
						key={index}
						href={link.href}
						className="flex flex-col items-center gap-3 px-4 py-6 bg-white/2 border border-white/20 rounded-xl hover:bg-white/5 hover:border-white/40 transition-all duration-200 group"
						target="_blank"
						rel="noopener noreferrer"
					>
						<Icon icon={link.icon} className="w-6 h-6 text-white" />
						<span className="text-white font-medium text-sm">
							{link.label}
						</span>
					</Link>
				))}
			</div>
		</div>
	);
}
