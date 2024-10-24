import { loadArticles } from "@/lib/article";
import { NextResponse } from "next/server";

export async function GET() {
    const articles = await loadArticles();

    const entries = articles.filter((article) =>
        article.category.id === "changelog"
    );

    const response = entries
        .sort((a, b) => b.published - a.published)
        .map((entry) => ({
            title: entry.title,
            description: entry.description,
            slug: entry.slug,
            published: entry.published,
            authors: [{
                ...entry.author,
                avatar: {
                    url: entry.author.avatar.src,
                    height: entry.author.avatar.height,
                    width: entry.author.avatar.width,
                },
            }],
            section: entry.category.name,
            tags: entry.tags,
            images: [
                {
                    url: entry.image.src,
                    width: entry.image.width,
                    height: entry.image.height,
                    alt: entry.image.alt,
                },
            ],
        }));

    return NextResponse.json(response);
}
