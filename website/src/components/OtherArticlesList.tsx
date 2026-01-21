import { getCollection } from 'astro:content';
import { AUTHORS } from '@/lib/article';

interface OtherArticlesListProps {
	currentSlug: string;
}

export const OtherArticlesList = async ({
	currentSlug,
}: OtherArticlesListProps) => {
	const posts = await getCollection('posts');

	// Filter out the current post, unpublished posts, and sort by date
	const otherPosts = posts
		.filter(post => {
			const slug = post.id.replace(/\/page$/, '');
			return slug !== currentSlug && !post.data.unpublished;
		})
		.sort((a, b) => b.data.published.getTime() - a.data.published.getTime());

	const articlesWithTitles = otherPosts.map((post) => {
		const slug = post.id.replace(/\/page$/, '');
		const author = AUTHORS[post.data.author];

		return {
			slug,
			title: post.data.title,
			author,
			date: post.data.published,
		};
	});

	const formatter = new Intl.DateTimeFormat("en", {});

	return (
		<ul className="mt-2 hidden text-sm text-cream-100 xl:block">
			{articlesWithTitles.map((article) => {
				return (
					<li key={article.slug} className="mb-3 flex">
						<a href={`/blog/${article.slug}/`} className="hover:text-cream-300">
							<p className="text-xs leading-tight">
								{article.title}
							</p>
							<div className="text-2xs text-charcole-800">
								{article.author.name} @{" "}
								<i>
									{formatter.format(article.date)}
								</i>
							</div>
						</a>
					</li>
				);
			})}
		</ul>
	);
};
