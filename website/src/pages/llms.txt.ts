import type { APIRoute } from 'astro';
import { getCollection, render } from 'astro:content';

export const prerender = true;

export const GET: APIRoute = async ({ site }) => {
	const siteUrl = site?.toString().replace(/\/$/, '') || 'https://rivet.dev';

	// Get all content collections
	const [docs, cookbook, guides, posts] = await Promise.all([
		getCollection('docs'),
		getCollection('cookbook'),
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

	// Build cookbook URLs
	const cookbookUrls = cookbook
		.map(entry => {
			const cleanPath = entry.id.replace(/\/index$/, '').replace(/^index$/, '');
			return cleanPath ? `${siteUrl}/cookbook/${cleanPath}/` : `${siteUrl}/cookbook/`;
		})
		.sort();

	// Build blog URLs (filter out unpublished)
	const blogUrls = posts
		.filter(post => !post.data.unpublished)
		.map(post => {
			const slug = post.id.replace(/\/page$/, '');
			return `${siteUrl}/blog/${slug}/`;
		})
		.sort();

	// Build changelog URLs (same posts, different path, filter out unpublished)
	const changelogUrls = posts
		.filter(post => post.data.category === 'changelog' && !post.data.unpublished)
		.map(post => {
			const slug = post.id.replace(/\/page$/, '');
			return `${siteUrl}/changelog/${slug}/`;
		})
		.sort();

	// Static site pages
	const staticUrls = [
		`${siteUrl}/`,
		`${siteUrl}/docs/`,
		`${siteUrl}/cookbook/`,
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
		...cookbookUrls,
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
