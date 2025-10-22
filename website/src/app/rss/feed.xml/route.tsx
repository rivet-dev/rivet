import { loadArticles } from "@/lib/article";
import { getSiteUrl } from "@/lib/siteUrl";

import { Feed } from "feed";
import { NextResponse } from "next/server";

export const dynamic = "force-static";

export async function GET() {
	const siteUrl = getSiteUrl();

	const articles = await loadArticles();

	const feed = new Feed({
		title: "Rivet",
		description: "Rivet news",
		id: siteUrl,
		link: siteUrl,
		image: `${siteUrl}/favicon.ico`,
		favicon: `${siteUrl}/favicon.ico`,
		copyright: `All rights reserved ${new Date().getFullYear()} Rivet Gaming, Inc.`,
		feedLinks: {
			rss2: `${siteUrl}/rss/feed.xml`,
		},
	});

	for (const article of articles) {
		const url = `${siteUrl}/blog/${article.slug}`;
		feed.addItem({
			title: article.title,
			id: article.slug,
			date: article.published,
			author: article.author.name,
			link: url,
			description: article.description,
		});
	}

	const response = new NextResponse(feed.rss2());
	response.headers.set("Content-Type", "application/xml");
	return response;
}
