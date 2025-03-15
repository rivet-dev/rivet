import { Button } from "@rivet-gg/components";
import { FancyHeader } from "@/components/v2/FancyHeader";

async function getOssFriends() {
	const res = await fetch("https://formbricks.com/api/oss-friends", {
		next: { revalidate: 3600 } // Revalidate every hour
	});
	const data = await res.json();
	return data.data;
}

export default async function OssFriendsPage() {
	const items = await getOssFriends();

	return (
		<>
			<FancyHeader />
			<div className="bg-black">
				<div className="mx-auto max-w-7xl px-6 py-24 sm:py-32 lg:px-8">
					<div className="mx-auto max-w-2xl text-center">
						<h1 className="text-4xl font-bold tracking-tight sm:text-6xl">
							{"Rivet's"}{" "}
							<span className="text-white">
								Open Source
							</span>{" "}
							Friends
						</h1>
						<p className="mt-6 text-lg leading-8 text-gray-300">
							Other companies whose code & culture mirrors that at Rivet.
						</p>
					</div>

					<div className="mx-auto mt-16 grid max-w-2xl grid-cols-1 gap-6 sm:mt-20 lg:mx-0 lg:max-w-none lg:grid-cols-2">
						{items.map((friend: any, index: number) => (
							<a
								key={index}
								href={friend.href}
								target="_blank"
								rel="noopener noreferrer"
								className="flex flex-col overflow-hidden bg-[#080808] p-6 border-2 border-[#2B2B2B] hover:border-white/30 transition-colors"
							>
								<div className="mb-2 font-display text-xl font-bold">
									{friend.name}
								</div>
								<p className="text-gray-300 mt-0 text-sm">
									{friend.description}
								</p>
							</a>
						))}
					</div>
				</div>
			</div>
		</>
	);
} 