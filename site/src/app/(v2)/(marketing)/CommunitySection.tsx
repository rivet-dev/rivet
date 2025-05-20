"use client";

import Link from "next/link";
import {
	Icon,
	faDiscord,
	faGithub,
	faTwitter,
	faBluesky,
    faXTwitter,
} from "@rivet-gg/icons";

// Community section
export const CommunitySection = () => {
	const platforms = [
		{ name: "Discord", icon: faDiscord, href: "https://discord.gg/aXYfyNxYVn" },
		{
			name: "Discussions",
			icon: faGithub,
			href: "https://github.com/rivet-gg/rivet/discussions",
		},
		{ name: "X", icon: faXTwitter, href: "https://x.com/rivet_gg" },
		{
			name: "Bluesky",
			icon: faBluesky,
			href: "https://bsky.app/profile/rivet.gg",
		},
	];

	// Tweets for each column
	const columnTweets = [
		// Column 1
		[
			{
				user: "Jane Doe",
				handle: "@janedoe",
				content:
					"Just deployed my first stateful job with @rivet_gg and I'm blown away by how simple it was. The documentation is excellent!",
			},
			{
				user: "Mike Williams",
				handle: "@mikew",
				content:
					"As someone new to serverless, Rivet made the learning curve much smoother. Their tutorials are super straightforward.",
			},
			{
				user: "Sara Chen",
				handle: "@sarac",
				content:
					"Been using @rivet_gg for our AI agents and the performance is incredible. Self-hosting was a breeze too!",
			},
		],
		// Column 2
		[
			{
				user: "John Smith",
				handle: "@johnsmith",
				content:
					"Rivet has completely changed how we handle our functions. Our team's productivity has doubled since we made the switch.",
			},
			{
				user: "Emma Rodriguez",
				handle: "@emmar",
				content:
					"The desktop sandbox feature in @rivet_gg is a game changer for our GUI-based applications. Nothing else comes close.",
			},
			{
				user: "David Kim",
				handle: "@davidk",
				content:
					"Our entire backend is running on Rivet now. The monitoring tools make my job 10x easier as a DevOps engineer.",
			},
		],
		// Column 3
		[
			{
				user: "Alex Johnson",
				handle: "@alexj",
				content:
					"I've tried all the serverless platforms out there, and @rivet_gg is hands down the best one for my needs. The open-source aspect sealed the deal for me.",
			},
			{
				user: "Taylor Morgan",
				handle: "@taylorm",
				content:
					"Moving from AWS to self-hosted @rivet_gg cut our costs by 70% while improving performance. Best decision we made this year.",
			},
			{
				user: "Jordan Lee",
				handle: "@jordanl",
				content:
					"The Rivet community is amazing! Got help with my implementation within minutes of posting a question.",
			},
		],
	];

	return (
		<div className="mx-auto max-w-7xl px-6 pt-16 pb-8 lg:pt-20 lg:pb-10 lg:px-8">
			{/* Header */}
			<div className="text-center mb-10">
				<h2 className="text-4xl font-medium tracking-tight text-white">
					Join the community
				</h2>
				<p className="mt-4 text-lg text-white/70">
					Connect with thousands of developers building with Rivet
				</p>
			</div>

			{/* Social links */}
			<div className="flex justify-center space-x-3 mb-16">
				{platforms.map((platform, index) => (
					<Link key={index} href={platform.href} className="group">
						<div className="flex items-center justify-center h-10 px-5 rounded-md border border-zinc-800 bg-zinc-900 group-hover:bg-zinc-800 group-hover:border-zinc-600 transition-all">
							<Icon
								icon={platform.icon}
								className="text-white/80 group-hover:text-white mr-2 transition-colors"
							/>
							<span className="text-sm text-white/80 group-hover:text-white transition-colors font-medium">
								{platform.name}
							</span>
						</div>
					</Link>
				))}
			</div>

			{/* Tweets */}
			{/*<div className="grid grid-cols-1 md:grid-cols-3 gap-8">
				{columnTweets.map((column, columnIndex) => (
					<div key={columnIndex} className="flex flex-col space-y-6">
						{column.map((tweet, tweetIndex) => (
							<div
								key={`${columnIndex}-${tweetIndex}`}
								className="rounded-xl p-6 bg-black/20 border border-white/5 flex flex-col hover:bg-black/30 hover:border-white/10 transition-all"
							>
								<div className="flex items-center mb-4">
									<div className="w-10 h-10 rounded-full bg-zinc-700 mr-3"></div>
									<div>
										<div className="text-white font-medium">
											{tweet.user}
										</div>
										<div className="text-white/50 text-sm">
											{tweet.handle}
										</div>
									</div>
								</div>
								<p className="text-white/80">{tweet.content}</p>
							</div>
						))}
					</div>
				))}
			</div>*/}
		</div>
	);
};
