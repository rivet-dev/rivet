import { defineCollection, z } from 'astro:content';
import { glob } from 'astro/loaders';

const docs = defineCollection({
	loader: glob({ pattern: '**/*.mdx', base: './src/content/docs' }),
	schema: z.object({
		title: z.string().optional(),
		description: z.string().optional(),
	}),
});

const guides = defineCollection({
	loader: glob({ pattern: '**/*.mdx', base: './src/content/guides' }),
	schema: z.object({
		title: z.string().optional(),
		description: z.string().optional(),
	}),
});

const learn = defineCollection({
	loader: glob({ pattern: '**/*.mdx', base: './src/content/learn' }),
	schema: z.object({
		title: z.string().optional(),
		description: z.string().optional(),
		act: z.string().optional(),
		subtitle: z.string().optional(),
	}),
});

const posts = defineCollection({
	loader: glob({ pattern: '**/page.mdx', base: './src/content/posts' }),
	schema: z.object({
		author: z.enum(['nathan-flurry', 'nicholas-kissel', 'forest-anderson']),
		published: z.coerce.date(),
		category: z.enum(['changelog', 'monthly-update', 'launch-week', 'technical', 'guide', 'frogs']),
		keywords: z.array(z.string()).optional(),
	}),
});

export const collections = {
	docs,
	guides,
	learn,
	posts,
};
