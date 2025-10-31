import { Icon, faArrowRight } from "@rivet-gg/icons";
import Image from "next/image";
import Link from "next/link";

import chinomanAvatar from "../images/quotes/users/Chinoman10_.jpg";
import socialQuotientAvatar from "../images/quotes/users/Social_Quotient.jpg";
import alistaiirAvatar from "../images/quotes/users/alistaiir.jpg";
// User avatars
import devgerredAvatar from "../images/quotes/users/devgerred.jpg";
import j0g1tAvatar from "../images/quotes/users/j0g1t.jpg";
import localFirstAvatar from "../images/quotes/users/localfirstnews.jpg";
import samgoodwinAvatar from "../images/quotes/users/samgoodwin89.jpg";
import samk0Avatar from "../images/quotes/users/samk0_com.jpg";
import uripontAvatar from "../images/quotes/users/uripont_.jpg";

import j0g1tPostImage from "../images/quotes/posts/1902835527977439591.jpg";
// Post images
import samk0PostImage from "../images/quotes/posts/1909278348812952007.png";

export function QuotesSection() {
	const quotesColumn1 = [
		{
			href: "https://x.com/devgerred/status/1903178025598083285",
			avatar: devgerredAvatar,
			name: "gerred",
			handle: "@devgerred",
			content: "Nice work, @rivet_dev - nailed it",
		},
		{
			href: "https://x.com/samk0_com/status/1909278348812952007",
			avatar: samk0Avatar,
			name: "Samo",
			handle: "@samk0_com",
			content: "Great UX & DX possible thanks to @RivetKit_org",
			image: samk0PostImage,
		},
		{
			href: "https://x.com/Social_Quotient/status/1903172142121832905",
			avatar: socialQuotientAvatar,
			name: "John Curtis",
			handle: "@Social_Quotient",
			content: "Loving RivetKit direction!",
		},
		{
			href: "https://x.com/localfirstnews/status/1902752173928427542",
			avatar: localFirstAvatar,
			name: "Local-First Newsletter",
			handle: "@localfirstnews",
			content: "Featured in newsletter",
			isItalic: true,
		},
		{
			href: "https://x.com/Chinoman10_/status/1902020312306216984",
			avatar: chinomanAvatar,
			name: "Chinomso",
			handle: "@Chinoman10_",
			content:
				"Alternatively, some dude (@NathanFlurry) recently told me about @RivetKit_org, which optionally brings you vendor-flexibility (no lock-in since it's abstracted for you).",
		},
	];

	const quotesColumn2 = [
		{
			href: "https://x.com/uripont_/status/1910817946470916525",
			avatar: uripontAvatar,
			name: "uripont",
			handle: "@uripont_",
			content:
				"Crazy to think that there are so many things to highlight that is actually hard to convey it in a few words.",
		},
		{
			href: "https://x.com/samgoodwin89/status/1910791029609091456",
			avatar: samgoodwinAvatar,
			name: "sam",
			handle: "@samgoodwin89",
			content: '"Durable Objects without the boilerplate"',
		},
		{
			href: "https://x.com/j0g1t/status/1902835527977439591",
			avatar: j0g1tAvatar,
			name: "Kacper Wojciechowski",
			handle: "@j0g1t",
			content:
				"Your outie uses @RivetKit_org to develop realtime applications.",
			image: j0g1tPostImage,
		},
		{
			href: "https://x.com/alistaiir/status/1891312940302716984",
			avatar: alistaiirAvatar,
			name: "alistair",
			handle: "@alistaiir",
			content: "RivetKit looks super awesome.",
		},
	];

	const QuoteCard = ({ quote }: { quote: any }) => (
		<Link
			href={quote.href}
			className="block p-6 bg-white/2 border border-white/20 rounded-xl hover:bg-white/10 hover:border-white/40 transition-all duration-200 group"
			target="_blank"
			rel="noopener noreferrer"
		>
			<div className="flex items-start gap-3 mb-4">
				<Image
					src={quote.avatar}
					alt={quote.name}
					width={40}
					height={40}
					className="rounded-full object-cover"
				/>
				<div className="flex-1 min-w-0">
					<p className="text-white font-medium text-sm">
						{quote.name}
					</p>
					<p className="text-white/40 text-sm">{quote.handle}</p>
				</div>
			</div>
			<p
				className={`text-white/40 font-500 leading-relaxed mb-4 ${quote.isItalic ? "italic" : ""}`}
			>
				{quote.content}
			</p>
			{quote.image && (
				<Image
					src={quote.image}
					alt="Tweet media"
					width={300}
					height={200}
					className="rounded-lg object-cover w-full"
				/>
			)}
		</Link>
	);

	return (
		<div className="mx-auto max-w-6xl">
			<div className="text-center mb-16">
				<h2 className="text-2xl sm:text-3xl font-700 text-white mb-6">
					What People Are Saying
				</h2>
				<p className="text-lg font-500 text-white/40">
					From the platform formerly known as Twitter
				</p>
			</div>

			<div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-16">
				{/* Column 1 */}
				<div className="space-y-6">
					{quotesColumn1.map((quote, index) => (
						<QuoteCard key={index} quote={quote} />
					))}
				</div>

				{/* Column 2 */}
				<div className="space-y-6">
					{quotesColumn2.map((quote, index) => (
						<QuoteCard key={index} quote={quote} />
					))}
				</div>
			</div>

			{/* Tweet Button */}
			<div className="text-center">
				<Link
					href="https://twitter.com/intent/tweet?text=%40RivetKit_org%20"
					className="inline-flex items-center gap-2 px-4 py-2 bg-white/2 border border-white/20 rounded-lg text-white font-medium hover:bg-white/10 hover:border-white/40 transition-all duration-200"
					target="_blank"
					rel="noopener noreferrer"
				>
					Share your feedback on X
					<Icon icon={faArrowRight} className="w-4 h-4" />
				</Link>
			</div>
		</div>
	);
}
