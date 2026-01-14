import type { APIRoute } from 'astro';
import { getCollection, render } from 'astro:content';

export const prerender = true;

export const GET: APIRoute = async ({ site }) => {
	const siteUrl = site?.toString().replace(/\/$/, '') || 'https://rivet.gg';

	// Get all content collections
	const [docs, guides, posts] = await Promise.all([
		getCollection('docs'),
		getCollection('guides'),
		getCollection('posts'),
	]);

	// Build docs URLs (exclude cloud docs)
	const docsUrls = docs
		.filter(doc => !doc.id.startsWith('cloud'))
		.map(doc => {
			const cleanPath = doc.id.replace(/\/index$/, '').replace(/^index$/, '');
			return cleanPath ? `${siteUrl}/docs/${cleanPath}/` : `${siteUrl}/docs/`;
		})
		.sort();

	// Build guides URLs
	const guidesUrls = guides
		.map(guide => `${siteUrl}/guides/${guide.id}/`)
		.sort();

	// Build blog URLs
	const blogUrls = posts
		.map(post => {
			const slug = post.id.replace(/\/page$/, '');
			return `${siteUrl}/blog/${slug}/`;
		})
		.sort();

	// Build changelog URLs (same posts, different path)
	const changelogUrls = posts
		.filter(post => post.data.category === 'changelog')
		.map(post => {
			const slug = post.id.replace(/\/page$/, '');
			return `${siteUrl}/changelog/${slug}/`;
		})
		.sort();

	// Static site pages
	const staticUrls = [
		`${siteUrl}/`,
		`${siteUrl}/docs/`,
		`${siteUrl}/templates/`,
		`${siteUrl}/cloud/`,
		`${siteUrl}/pricing/`,
		`${siteUrl}/changelog/`,
		`${siteUrl}/blog/`,
		`${siteUrl}/support/`,
		`${siteUrl}/talk-to-an-engineer/`,
		`${siteUrl}/sales/`,
		`${siteUrl}/oss-friends/`,
		`${siteUrl}/terms/`,
		`${siteUrl}/privacy/`,
		`${siteUrl}/acceptable-use/`,
		`${siteUrl}/rss/feed.xml`,
		`${siteUrl}/changelog.json`,
	].sort();

	// Combine all URLs
	const allUrls = [...new Set([
		...staticUrls,
		...docsUrls,
		...guidesUrls,
		...blogUrls,
		...changelogUrls,
	])].sort();

	const content = [
		'# Rivet Documentation Index',
		'',
		...allUrls
	].join('\n');

	return new Response(content, {
		headers: {
			'Content-Type': 'text/plain; charset=utf-8',
		},
	});
};
