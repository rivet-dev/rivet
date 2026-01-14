# Next.js to Astro Migration Spec

This document specifies how to migrate `website/` from Next.js to `website-astro/` using Astro.

---

## CRITICAL: Static-Only Rendering

**The entire site MUST be pre-rendered at build time.** No server-side rendering (SSR) is allowed. The site will be served as static files via Caddy, exactly like the current Next.js setup.

**Requirements:**
- `output: 'static'` in astro.config.mjs (this is the default)
- All pages must use `getStaticPaths()` for dynamic routes
- No `server` output mode
- No `hybrid` rendering
- API routes (RSS, JSON feeds) must be pre-rendered at build time
- Build outputs to `dist/` directory, served by Caddy

---

## CRITICAL: Route Parity Requirements

**All routes MUST be 1:1 with the existing Next.js website.** No routes should be added, removed, or renamed. URL structure must be identical to preserve SEO, existing links, and user bookmarks.

### Required Route Structure

The following routes must be implemented exactly as specified:

#### Marketing Pages
| Next.js Path | URL | Astro Path |
|--------------|-----|------------|
| `(v2)/(marketing)/(index)/page.tsx` | `/` | `src/pages/index.astro` |
| `(v2)/(marketing)/agent/page.tsx` | `/agent/` | `src/pages/agent.astro` |
| `(v2)/(marketing)/cloud/page.tsx` | `/cloud/` | `src/pages/cloud.astro` |
| `(v2)/(marketing)/pricing/page.tsx` | `/pricing/` | `src/pages/pricing.astro` (redirect to /cloud/) |
| `(v2)/(marketing)/sales/page.tsx` | `/sales/` | `src/pages/sales.astro` |
| `(v2)/(marketing)/support/page.tsx` | `/support/` | `src/pages/support.astro` |
| `(v2)/(marketing)/startups/page.tsx` | `/startups/` | `src/pages/startups.astro` |
| `(v2)/(marketing)/talk-to-an-engineer/page.tsx` | `/talk-to-an-engineer/` | `src/pages/talk-to-an-engineer.astro` |
| `(v2)/(marketing)/rivet-vs-cloudflare-workers/page.tsx` | `/rivet-vs-cloudflare-workers/` | `src/pages/rivet-vs-cloudflare-workers.astro` |
| `(v2)/oss-friends/page.tsx` | `/oss-friends/` | `src/pages/oss-friends.astro` |

#### Solutions Pages
| Next.js Path | URL | Astro Path |
|--------------|-----|------------|
| `(v2)/(marketing)/solutions/agents/page.tsx` | `/solutions/agents/` | `src/pages/solutions/agents.astro` |
| `(v2)/(marketing)/solutions/collaborative-state/page.tsx` | `/solutions/collaborative-state/` | `src/pages/solutions/collaborative-state.astro` |
| `(v2)/(marketing)/solutions/game-servers/page.tsx` | `/solutions/game-servers/` | `src/pages/solutions/game-servers.astro` |
| `(v2)/(marketing)/solutions/games/page.tsx` | `/solutions/games/` | `src/pages/solutions/games.astro` |
| `(v2)/(marketing)/solutions/user-session-store/page.tsx` | `/solutions/user-session-store/` | `src/pages/solutions/user-session-store.astro` |
| `(v2)/(marketing)/solutions/workflows/page.tsx` | `/solutions/workflows/` | `src/pages/solutions/workflows.astro` |

#### Templates Pages
| Next.js Path | URL | Astro Path |
|--------------|-----|------------|
| `(v2)/(marketing)/templates/page.tsx` | `/templates/` | `src/pages/templates/index.astro` |
| `(v2)/(marketing)/templates/[slug]/page.tsx` | `/templates/[slug]/` | `src/pages/templates/[slug].astro` |

#### Blog & Changelog
| Next.js Path | URL | Astro Path |
|--------------|-----|------------|
| `(v2)/(blog)/blog/page.tsx` | `/blog/` | `src/pages/blog/index.astro` |
| `(v2)/(blog)/blog/[...slug]/page.tsx` | `/blog/[...slug]/` | `src/pages/blog/[...slug].astro` |
| `(v2)/(blog)/changelog/page.tsx` | `/changelog/` | `src/pages/changelog/index.astro` |
| `(v2)/(blog)/changelog/[...slug]/page.tsx` | `/changelog/[...slug]/` | `src/pages/changelog/[...slug].astro` |

#### Documentation (Dynamic Catch-All)
| Next.js Path | URL | Astro Path |
|--------------|-----|------------|
| `(v2)/[section]/[[...page]]/page.tsx` | `/docs/`, `/docs/**` | `src/pages/docs/[...slug].astro` |
| `(v2)/[section]/[[...page]]/page.tsx` | `/guides/`, `/guides/**` | `src/pages/guides/[...slug].astro` |
| `(v2)/learn/[[...page]]/page.tsx` | `/learn/`, `/learn/**` | `src/pages/learn/[...slug].astro` |

#### Legal/Content Pages (MDX)
| Next.js Path | URL | Astro Path |
|--------------|-----|------------|
| `(v2)/(content)/terms/page.mdx` | `/terms/` | `src/pages/terms.astro` or `src/pages/terms.mdx` |
| `(v2)/(content)/privacy/page.mdx` | `/privacy/` | `src/pages/privacy.astro` or `src/pages/privacy.mdx` |
| `(v2)/(content)/acceptable-use/page.mdx` | `/acceptable-use/` | `src/pages/acceptable-use.astro` or `src/pages/acceptable-use.mdx` |

#### Redirect/Tool Pages
| Next.js Path | URL | Astro Path |
|--------------|-----|------------|
| `(v2)/(content)/docs/tools/[tool]/page.tsx` | `/docs/tools/[tool]/` | `src/pages/docs/tools/[tool].astro` (redirect to /docs/[tool]/) |

#### API/Feed Endpoints
| Next.js Path | URL | Astro Path |
|--------------|-----|------------|
| `rss/feed.xml/route.tsx` | `/rss/feed.xml` | `src/pages/rss/feed.xml.ts` |
| `(v2)/(blog)/changelog.json/route.ts` | `/changelog.json` | `src/pages/changelog.json.ts` |

#### Miscellaneous
| Next.js Path | URL | Astro Path |
|--------------|-----|------------|
| `(v2)/(other)/meme/wired-in/page.jsx` | `/meme/wired-in/` | `src/pages/meme/wired-in.astro` |

**Total: 28 route files to migrate**

---

## Uncertain Areas & Research Needed

### Areas Requiring Further Investigation

1. **`recmaPlugins` Support**
   - Current Next.js config uses recma plugins (currently empty array)
   - Astro MDX integration may not support recma directly
   - **Resolution:** Since the array is empty, this is not blocking

2. **`mdxAnnotations` Plugin Compatibility**
   - Custom annotation syntax used in MDX files
   - Need to verify it works with Astro's MDX processing
   - **Action:** Test during migration, may need adjustment

3. **Custom Shiki Theme**
   - Located at `src/lib/textmate-code-theme`
   - Need to ensure Astro's Shiki integration accepts custom themes
   - **Action:** Import and configure in astro.config.mjs

4. **`transformerTemplateVariables` Custom Transformer**
   - Custom Shiki transformer in `src/mdx/transformers.ts`
   - Used for autofill code blocks
   - **Action:** Verify compatibility with Astro's Shiki

5. **React Components with `"use client"`**
   - Several marketing pages use `"use client"` directive
   - Astro uses `client:load`, `client:visible`, etc.
   - **Action:** Audit all client components and add appropriate directives

6. **`@rivet-gg/components` and `@rivet-gg/icons` Packages**
   - Workspace packages used throughout
   - Should work as-is but need to verify build process
   - **Action:** Ensure workspace dependencies are properly linked

7. **Client-Side React Components**
   - Many marketing pages use `"use client"` with React hooks (useState, useEffect)
   - Examples: `/solutions/agents/`, `/solutions/workflows/`
   - **Action:** Keep as `.tsx` files with `client:load` directive, or convert to Astro with islands

8. **ScrollObserver Component**
   - Used on home page for scroll-based animations
   - Requires client-side JavaScript
   - **Action:** Keep as React component with `client:load`

---

## Table of Contents

1. [Project Structure](#project-structure)
2. [Content Collections Setup](#content-collections-setup)
3. [MDX Migration](#mdx-migration)
4. [Dynamic Routes Migration](#dynamic-routes-migration)
5. [Data Files Migration](#data-files-migration)
6. [Sitemap Integration](#sitemap-integration)
7. [llms.txt Generation](#llmstxt-generation)
8. [Railway Deployment](#railway-deployment)
9. [Copy Commands](#copy-commands)
10. [Example Migrations](#example-migrations)

---

## Project Structure

### Current Next.js Structure

```
website/
├── src/
│   ├── app/(v2)/                    # App Router pages
│   │   ├── [section]/[[...page]]/   # Docs/guides catch-all
│   │   ├── learn/[[...page]]/       # Learn section
│   │   ├── (blog)/blog/[...slug]/   # Blog posts
│   │   ├── (blog)/changelog/[...slug]/
│   │   └── (marketing)/templates/[slug]/
│   ├── content/                     # MDX content
│   │   ├── docs/                    # 68 MDX files
│   │   ├── guides/
│   │   └── learn/
│   ├── posts/                       # Blog posts (31 directories)
│   ├── components/
│   ├── data/                        # Static data files
│   ├── lib/
│   ├── mdx/                         # MDX plugins
│   └── sitemap/                     # Navigation config
├── public/
├── next.config.ts
└── package.json
```

### Target Astro Structure

```
website-astro/
├── src/
│   ├── pages/                       # Astro pages (file-based routing)
│   │   ├── docs/
│   │   │   └── [...slug].astro      # Catch-all for docs
│   │   ├── guides/
│   │   │   └── [...slug].astro      # Catch-all for guides
│   │   ├── learn/
│   │   │   └── [...slug].astro      # Learn section
│   │   ├── blog/
│   │   │   ├── index.astro          # Blog listing
│   │   │   └── [...slug].astro      # Blog posts
│   │   ├── changelog/
│   │   │   ├── index.astro
│   │   │   └── [...slug].astro
│   │   ├── templates/
│   │   │   ├── index.astro
│   │   │   └── [slug].astro
│   │   └── index.astro              # Home page
│   ├── content/                     # Content collections
│   │   ├── docs/                    # MDX docs
│   │   ├── guides/
│   │   ├── learn/
│   │   └── posts/                   # Blog posts (moved from src/posts)
│   ├── components/                  # Astro/React components
│   ├── data/                        # Static data files
│   ├── lib/                         # Utilities
│   ├── layouts/                     # Layout components
│   │   ├── BaseLayout.astro
│   │   ├── DocsLayout.astro
│   │   └── BlogLayout.astro
│   └── styles/                      # Global styles
├── public/
├── astro.config.mjs
├── content.config.ts                # Content collections config
├── tailwind.config.mjs
└── package.json
```

---

## Content Collections Setup

### content.config.ts

Create `src/content.config.ts` to define all content collections:

```typescript
import { defineCollection, z, reference } from 'astro:content';
import { glob } from 'astro/loaders';

// Docs collection (for /docs/* and /guides/*)
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

// Blog posts collection
const posts = defineCollection({
  loader: glob({ pattern: '**/page.mdx', base: './src/content/posts' }),
  schema: ({ image }) => z.object({
    author: z.enum(['nathan-flurry', 'nicholas-kissel', 'forest-anderson']),
    published: z.string().transform((str) => new Date(str)),
    category: z.enum(['changelog', 'monthly-update', 'launch-week', 'technical', 'guide', 'frogs']),
    keywords: z.array(z.string()).optional(),
    // Image will be handled separately via glob import
  }),
});

export const collections = {
  docs,
  guides,
  learn,
  posts,
};
```

### Key Differences from Next.js

| Next.js Pattern | Astro Equivalent |
|-----------------|------------------|
| `export const title = "..."` in MDX | YAML frontmatter `title: "..."` |
| Dynamic `import()` at runtime | `getCollection()` / `getEntry()` at build time |
| `useMDXComponents()` hook | `components` prop on `<Content />` |
| `generateStaticParams()` | `getStaticPaths()` |
| `generateMetadata()` | Frontmatter + Layout `<head>` |

---

## MDX Migration

### Front Matter Conversion

**Current Next.js MDX (JavaScript exports):**
```mdx
export const author = "nicholas-kissel"
export const published = "2024-12-21"
export const category = "changelog"
export const keywords = ["Actors"]

# Rivet Actors Launch

Content here...
```

**Target Astro MDX (YAML frontmatter):**
```mdx
---
author: nicholas-kissel
published: "2024-12-21"
category: changelog
keywords:
  - Actors
---

# Rivet Actors Launch

Content here...
```

### Rehype/Remark Plugin Migration

The existing plugins in `src/mdx/` can be reused with Astro's MDX integration. Remark and rehype plugins should be installed, imported, and applied as functions rather than strings.

**astro.config.mjs:**
```javascript
import { defineConfig } from 'astro/config';
import mdx from '@astrojs/mdx';
import react from '@astrojs/react';
import tailwind from '@astrojs/tailwind';
import sitemap from '@astrojs/sitemap';

// Import existing plugins
import { remarkPlugins } from './src/mdx/remark';
import { rehypePlugins } from './src/mdx/rehype';

export default defineConfig({
  site: 'https://www.rivet.dev',
  integrations: [
    mdx({
      // Inherit markdown config (default: true)
      extendMarkdownConfig: true,

      // Syntax highlighting
      syntaxHighlight: 'shiki',
      shikiConfig: {
        theme: 'github-dark',
        langs: [
          'bash', 'typescript', 'javascript', 'json', 'yaml',
          'rust', 'html', 'css', 'docker', 'toml',
        ],
      },

      // Remark plugins (process markdown AST)
      remarkPlugins,

      // Rehype plugins (process HTML AST)
      rehypePlugins,

      // Enable GitHub Flavored Markdown
      gfm: true,

      // Optimize build (disable for debugging)
      optimize: true,
    }),
    react(),
    tailwind(),
    sitemap({
      filter: (page) => !page.includes('/api/'),
    }),
  ],
  output: 'static',
  trailingSlash: 'always',
});
```

### MDX Plugin Configuration Details

**Current plugins that need migration:**

| Plugin | Purpose | Astro Compatibility |
|--------|---------|---------------------|
| `remarkGfm` | GitHub Flavored Markdown | Built-in via `gfm: true` |
| `mdxAnnotations` | Custom annotation syntax | Works as-is |
| `rehypeShiki` | Syntax highlighting | Use Astro's built-in or keep custom |
| `rehypeSlugify` | Heading IDs | Works as-is |
| `rehypeMdxTitle` | Extract title | Works, but consider using `headings` |
| `rehypeTableOfContents` | Generate TOC | Replace with Astro's `headings` |
| `rehypeDescription` | Extract description | Works as-is |

**Recommended changes:**

1. **Remove `rehypeTableOfContents`** - Astro's `render()` returns `headings` array
2. **Consider removing `rehypeMdxTitle`** - Use `headings[0]` from render result
3. **Keep custom Shiki config** or use Astro's built-in highlighting

### MDX Components Registration

**Current Next.js (`src/mdx-components.jsx`):**
```jsx
import * as mdx from "@/components/mdx";
export function useMDXComponents(components) {
  return { ...components, ...mdx };
}
```

**Astro approach - pass components to `<Content />`:**
```astro
---
import { getEntry, render } from 'astro:content';
import * as mdxComponents from '@/components/mdx';

const entry = await getEntry('docs', Astro.params.slug);
const { Content, headings } = await render(entry);
---

<Content components={mdxComponents} />
```

### MDX Component Conversions

Components used in MDX need to be converted or wrapped:

| Next.js Component | Astro Equivalent |
|-------------------|------------------|
| `<Link href="...">` (next/link) | `<a href="...">` |
| `<Image />` (next/image) | `<Image />` from `astro:assets` |
| `className` | `class` |
| `style={{ color: 'red' }}` | `style="color: red;"` |
| `{children}` | `<slot />` |

**src/components/mdx.ts (Astro version):**
```typescript
// Re-export components for MDX
export { default as Heading } from './Heading.astro';
export { default as SchemaPreview } from './SchemaPreview.astro';
export { default as Lead } from './Lead.astro';

// Keep React components that need interactivity
export { pre, code, CodeGroup, Code } from './v2/Code';

// Re-export from component library
export * from '@rivet-gg/components/mdx';
export { Resource } from './Resources';
export { Summary } from './Summary';
export { Accordion, AccordionGroup } from './Accordion';
export { Frame } from './Frame';
export { Card, CardGroup } from './Card';

// Standard HTML element overrides
export const a = (props: any) => <a {...props} />;
export const table = (props: any) => (
  <div class="overflow-x-auto">
    <table {...props} />
  </div>
);
```

### Automatic Exports (title, description, tableOfContents)

The current rehype plugins (`rehypeMdxTitle`, `rehypeDescription`, `rehypeTableOfContents`) inject exports into MDX. In Astro:

1. **Title**: Use `rehype-mdx-title` or extract from `headings` returned by `render()`
2. **Description**: First paragraph extraction stays the same, but store in frontmatter or compute at render time
3. **Table of Contents**: Use the `headings` array returned by `render()` instead of custom export

**Alternative: Keep plugins but access differently:**
```astro
---
const { Content, headings } = await render(entry);
// headings array: [{ depth: 2, slug: 'section', text: 'Section' }, ...]
---
```

---

## Dynamic Routes Migration

### Pattern: `generateStaticParams` → `getStaticPaths`

#### 1. Docs/Guides Catch-All Route

**Current Next.js (`src/app/(v2)/[section]/[[...page]]/page.tsx`):**
```typescript
export async function generateStaticParams() {
  const staticParams: Param[] = [];
  for (const section of VALID_SECTIONS) {
    const dir = path.join(process.cwd(), "src", "content", section);
    const dirs = await fs.readdir(dir, { recursive: true });
    const files = dirs.filter((file) => file.endsWith(".mdx"));
    const sectionParams = files.map((file) => createParamsForFile(section, file));
    staticParams.push(...sectionParams);
  }
  return staticParams;
}

export async function generateMetadata({ params }) {
  const { section, page } = await params;
  const { component: { title, description } } = await loadContent(path);
  return { title: `${title} - Rivet`, description };
}
```

**Target Astro (`src/pages/docs/[...slug].astro`):**
```astro
---
import { getCollection, render } from 'astro:content';
import DocsLayout from '@/layouts/DocsLayout.astro';
import * as mdxComponents from '@/components/mdx';

export async function getStaticPaths() {
  const docs = await getCollection('docs');
  return docs.map((entry) => ({
    params: { slug: entry.id },
    props: { entry },
  }));
}

const { entry } = Astro.props;
const { Content, headings } = await render(entry);

// Extract title from first h1 heading or frontmatter
const title = entry.data.title || headings.find(h => h.depth === 1)?.text || 'Documentation';
const description = entry.data.description || '';
---

<DocsLayout title={title} description={description} headings={headings}>
  <Content components={mdxComponents} />
</DocsLayout>
```

#### 2. Blog Posts Route

**Current Next.js (`src/app/(v2)/(blog)/blog/[...slug]/page.tsx`):**
```typescript
export function generateStaticParams() {
  return generateArticlesPageParams();
}

export async function generateMetadata({ params }) {
  const { slug } = await params;
  const { title, description, author, published, category, image } = await loadArticle(slug.join("/"));
  return {
    title,
    description,
    openGraph: { type: "article", publishedTime: published.toISOString(), ... },
  };
}
```

**Target Astro (`src/pages/blog/[...slug].astro`):**
```astro
---
import { getCollection, render } from 'astro:content';
import BlogLayout from '@/layouts/BlogLayout.astro';
import { AUTHORS, CATEGORIES } from '@/lib/article';
import * as mdxComponents from '@/components/mdx';

export async function getStaticPaths() {
  const posts = await getCollection('posts');
  return posts.map((entry) => {
    // entry.id will be like "2024-12-21-rivet-actors-launch/page"
    // Transform to slug format
    const slug = entry.id.replace(/\/page$/, '');
    return {
      params: { slug },
      props: { entry },
    };
  });
}

const { entry } = Astro.props;
const { Content, headings } = await render(entry);

const author = AUTHORS[entry.data.author];
const category = CATEGORIES[entry.data.category];

// Load image (co-located in content folder)
const images = import.meta.glob('/src/content/posts/*/image.{png,jpg,gif}', { eager: true });
const imagePath = Object.keys(images).find(p => p.includes(entry.id.replace('/page', '')));
const image = imagePath ? images[imagePath] : null;
---

<BlogLayout
  title={headings.find(h => h.depth === 1)?.text || 'Blog Post'}
  description={entry.data.description}
  author={author}
  published={entry.data.published}
  category={category}
  image={image}
>
  <Content components={mdxComponents} />
</BlogLayout>
```

#### 3. Templates Route (Data-driven)

**Current Next.js (`src/app/(v2)/(marketing)/templates/[slug]/page.tsx`):**
```typescript
export async function generateStaticParams() {
  return templates.map((template) => ({ slug: template.name }));
}
```

**Target Astro (`src/pages/templates/[slug].astro`):**
```astro
---
import { templates } from '@/data/templates/shared';
import TemplateLayout from '@/layouts/TemplateLayout.astro';

export async function getStaticPaths() {
  return templates.map((template) => ({
    params: { slug: template.name },
    props: { template },
  }));
}

const { template } = Astro.props;
---

<TemplateLayout template={template}>
  <!-- Template content -->
</TemplateLayout>
```

#### 4. Learn Section Route

**Current Next.js (`src/app/(v2)/learn/[[...page]]/page.tsx`):**
```typescript
export async function generateStaticParams(): Promise<{ page: string[] }[]> {
  const files = await fs.readdir(dir, { recursive: true });
  const mdxFiles = files.filter((file) => file.endsWith(".mdx"));
  return mdxFiles.map((file) => {
    const segments = file.replace(".mdx", "").split("/").filter(Boolean);
    return { page: segments };
  });
}
```

**Target Astro (`src/pages/learn/[...slug].astro`):**
```astro
---
import { getCollection, render } from 'astro:content';
import LearnLayout from '@/layouts/LearnLayout.astro';
import * as mdxComponents from '@/components/mdx';

export async function getStaticPaths() {
  const learn = await getCollection('learn');
  return learn.map((entry) => ({
    params: { slug: entry.id || undefined },
    props: { entry },
  }));
}

const { entry } = Astro.props;
const { Content, headings } = await render(entry);
---

<LearnLayout
  title={entry.data.title}
  act={entry.data.act}
  subtitle={entry.data.subtitle}
  headings={headings}
>
  <Content components={mdxComponents} />
</LearnLayout>
```

---

## Data Files Migration

### data/templates/shared.ts

No changes needed - re-exports from `@rivetkit/example-registry`:
```typescript
export {
  TECHNOLOGIES,
  TAGS,
  templates,
  type Technology,
  type Tag,
  type Template,
} from "@rivetkit/example-registry";
```

### data/use-cases.ts

No changes needed - static TypeScript data file.

### data/deploy/shared.ts

No changes needed - static deployment options.

### lib/article.tsx → lib/article.ts

**Changes needed:**
1. Remove dynamic `import()` calls - use content collections instead
2. Keep `AUTHORS` and `CATEGORIES` constants
3. Remove `loadArticle`, `loadArticles`, `generateArticlesPageParams` - replaced by `getCollection()`

**New lib/article.ts:**
```typescript
import nathanFlurry from '@/authors/nathan-flurry/avatar.jpeg';
import nicholasKissel from '@/authors/nicholas-kissel/avatar.jpeg';
import forestAnderson from '@/authors/forest-anderson/avatar.jpeg';

export const AUTHORS = {
  "nathan-flurry": {
    name: "Nathan Flurry",
    role: "Co-founder & CTO",
    avatar: nathanFlurry,
    socials: {
      twitter: "https://x.com/NathanFlurry/",
      github: "https://github.com/nathanflurry",
      bluesky: "https://bsky.app/profile/nathanflurry.com",
    },
  },
  "nicholas-kissel": {
    name: "Nicholas Kissel",
    role: "Co-founder & CEO",
    avatar: nicholasKissel,
    socials: {
      twitter: "https://x.com/NicholasKissel",
      github: "https://github.com/nicholaskissel",
      bluesky: "https://bsky.app/profile/nicholaskissel.com",
    },
  },
  "forest-anderson": {
    name: "Forest Anderson",
    role: "Founding Engineer",
    avatar: forestAnderson,
    url: "https://twitter.com/angelonfira",
  },
} as const;

export const CATEGORIES = {
  changelog: { name: "Changelog" },
  "monthly-update": { name: "Monthly Update" },
  "launch-week": { name: "Launch Week" },
  technical: { name: "Technical" },
  guide: { name: "Guide" },
  frogs: { name: "Frogs" },
} as const;

export type AuthorId = keyof typeof AUTHORS;
export type CategoryId = keyof typeof CATEGORIES;
```

---

## Marketing Pages Migration

Marketing pages are React components that need conversion to Astro. Many use client-side interactivity.

### Home Page (`/`)

**Current:** `src/app/(v2)/(marketing)/(index)/page.tsx`
- Uses multiple section components (RedesignedHero, StatsSection, etc.)
- `ScrollObserver` wraps entire page for scroll-based animations
- Fetches latest changelog title at build time

**Target:** `src/pages/index.astro`
```astro
---
import BaseLayout from '@/layouts/BaseLayout.astro';
import { getCollection } from 'astro:content';

// Section components (keep as React with client:load for interactive ones)
import { RedesignedHero } from '@/components/home/RedesignedHero';
import { StatsSection } from '@/components/home/StatsSection';
import { ConceptSection } from '@/components/home/ConceptSection';
// ... other sections

// Get latest changelog title
const posts = await getCollection('posts');
const changelogEntries = posts.filter(p => p.data.category === 'changelog');
const latest = changelogEntries.sort((a, b) =>
  b.data.published.getTime() - a.data.published.getTime()
)[0];

// Extract h1 from MDX content
import { render } from 'astro:content';
const { headings } = await render(latest);
const latestChangelogTitle = headings.find(h => h.depth === 1)?.text || '';
---

<BaseLayout title="Rivet - Stateful Serverless Platform">
  <div class="min-h-screen bg-black font-sans text-zinc-300">
    <main>
      <RedesignedHero client:load latestChangelogTitle={latestChangelogTitle} />
      <StatsSection client:visible />
      <ConceptSection />
      <!-- Non-interactive sections can be Astro components -->
    </main>
  </div>
</BaseLayout>
```

### Solutions Pages (Client-Side Heavy)

Pages like `/solutions/agents/` are fully client-rendered with `"use client"`.

**Strategy:** Keep as React components, wrap with Astro layout:

```astro
---
// src/pages/solutions/agents.astro
import BaseLayout from '@/layouts/BaseLayout.astro';
import AgentsPage from '@/components/solutions/AgentsPage';
---

<BaseLayout title="AI Agents - Rivet">
  <AgentsPage client:load />
</BaseLayout>
```

### Simple Marketing Pages

Pages like `/sales/`, `/support/` that are mostly static can be converted to pure Astro.

### Redirect Pages

Some pages are simple redirects (e.g., `/pricing/` → `/cloud/`):

```astro
---
// src/pages/pricing.astro
return Astro.redirect('/cloud/', 301);
---
```

Or use `astro.config.mjs` redirects:
```javascript
export default defineConfig({
  redirects: {
    '/pricing': '/cloud/',
  },
});
```

---

## API Routes Migration

### RSS Feed (`/rss/feed.xml`)

**Current:** `src/app/rss/feed.xml/route.tsx`

**Target:** `src/pages/rss/feed.xml.ts`

**Important:** For static pre-rendering, use `export const prerender = true;` (though this is the default for static output mode).

```typescript
import type { APIRoute } from 'astro';
import { getCollection } from 'astro:content';
import { Feed } from 'feed';
import { AUTHORS, CATEGORIES } from '@/lib/article';

// Ensure this route is pre-rendered at build time
export const prerender = true;

export const GET: APIRoute = async ({ site }) => {
  const siteUrl = site?.toString() || 'https://www.rivet.dev';
  const posts = await getCollection('posts');

  const feed = new Feed({
    title: 'Rivet',
    description: 'Rivet news',
    id: siteUrl,
    link: siteUrl,
    image: `${siteUrl}/favicon.ico`,
    favicon: `${siteUrl}/favicon.ico`,
    copyright: `All rights reserved ${new Date().getFullYear()} Rivet Gaming, Inc.`,
    feedLinks: {
      rss2: `${siteUrl}/rss/feed.xml`,
    },
  });

  for (const post of posts) {
    const slug = post.id.replace(/\/page$/, '');
    const url = `${siteUrl}/blog/${slug}`;
    const author = AUTHORS[post.data.author];

    feed.addItem({
      title: post.data.title || slug,
      id: slug,
      date: post.data.published,
      author: [{ name: author.name }],
      link: url,
      description: post.data.description || '',
    });
  }

  return new Response(feed.rss2(), {
    headers: {
      'Content-Type': 'application/xml; charset=utf-8',
    },
  });
};
```

### Changelog JSON (`/changelog.json`)

**Current:** `src/app/(v2)/(blog)/changelog.json/route.ts`

**Target:** `src/pages/changelog.json.ts`

**Important:** Must be pre-rendered at build time for static hosting.

```typescript
import type { APIRoute } from 'astro';
import { getCollection, render } from 'astro:content';
import { AUTHORS, CATEGORIES } from '@/lib/article';

// Ensure this route is pre-rendered at build time
export const prerender = true;

export const GET: APIRoute = async () => {
  const posts = await getCollection('posts');
  const changelogPosts = posts.filter(p => p.data.category === 'changelog');

  const entries = await Promise.all(
    changelogPosts
      .sort((a, b) => b.data.published.getTime() - a.data.published.getTime())
      .map(async (entry) => {
        const author = AUTHORS[entry.data.author];
        const { headings } = await render(entry);
        const title = headings.find(h => h.depth === 1)?.text || entry.id;

        return {
          title,
          description: entry.data.description || '',
          slug: entry.id.replace(/\/page$/, ''),
          published: entry.data.published,
          authors: [{
            name: author.name,
            role: author.role,
            avatar: {
              url: author.avatar.src,
              height: author.avatar.height,
              width: author.avatar.width,
            },
          }],
          section: CATEGORIES[entry.data.category].name,
          tags: entry.data.keywords || [],
        };
      })
  );

  return new Response(JSON.stringify(entries), {
    headers: {
      'Content-Type': 'application/json; charset=utf-8',
    },
  });
};
```

---

## Layout Migration

Next.js uses nested layouts via route groups. Astro uses explicit layout imports.

### Layout Hierarchy

```
Next.js                          Astro
─────────────────────────────────────────────────────
app/layout.tsx                   src/layouts/RootLayout.astro
└── (v2)/layout.tsx              src/layouts/BaseLayout.astro (includes Footer)
    ├── (marketing)/layout.tsx   src/layouts/MarketingLayout.astro (includes Header)
    ├── (blog)/layout.tsx        src/layouts/BlogLayout.astro
    ├── (content)/layout.tsx     src/layouts/ContentLayout.astro (prose styling)
    ├── [section]/layout.tsx     src/layouts/DocsLayout.astro
    └── learn/layout.tsx         src/layouts/LearnLayout.astro
```

### BaseLayout.astro (Root)

```astro
---
import '@/styles/main.css';
import { Footer } from '@/components/Footer';
import { EmbedDetector } from '@/components/EmbedDetector';

interface Props {
  title: string;
  description?: string;
  canonicalUrl?: string;
  ogImage?: string;
}

const { title, description, canonicalUrl, ogImage } = Astro.props;
---

<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>{title}</title>
    {description && <meta name="description" content={description} />}
    {canonicalUrl && <link rel="canonical" href={canonicalUrl} />}
    {ogImage && <meta property="og:image" content={ogImage} />}
    <link rel="icon" href="/favicon.ico" />
  </head>
  <body>
    <slot />
    <EmbedDetector client:load />
    <Footer />
  </body>
</html>
```

### MarketingLayout.astro

```astro
---
import BaseLayout from './BaseLayout.astro';
import { Header } from '@/components/v2/Header';

interface Props {
  title: string;
  description?: string;
}

const { title, description } = Astro.props;
---

<BaseLayout title={title} description={description}>
  <Header variant="floating" client:load />
  <slot />
</BaseLayout>
```

---

## Legal/Static MDX Pages

Pages like `/terms/`, `/privacy/`, `/acceptable-use/` are MDX files rendered with prose styling.

### Option 1: Keep as MDX in pages

```mdx
---
// src/pages/terms.mdx
layout: '@/layouts/ContentLayout.astro'
title: 'Terms of Service'
---

# Terms of Service

Content here...
```

### Option 2: Use content collection

Move to `src/content/legal/` and create a catch-all route, but this adds complexity for just 3 pages.

### Recommended: Direct MDX pages

Keep simple - use MDX files directly in `src/pages/`:
- `src/pages/terms.mdx`
- `src/pages/privacy.mdx`
- `src/pages/acceptable-use.mdx`

Each with layout frontmatter pointing to ContentLayout.

---

## Redirect Pages Migration

### Tools Redirect (`/docs/tools/[tool]/`)

**Current behavior:** Redirects `/docs/tools/actors/` → `/docs/actors/`

**Target:** `src/pages/docs/tools/[tool].astro`
```astro
---
export function getStaticPaths() {
  return [{ params: { tool: 'actors' } }];
}

const { tool } = Astro.params;
return Astro.redirect(`/docs/${tool}/`, 301);
---
```

### Meme Page (`/meme/wired-in/`)

Simple iframe page - convert directly:

**Target:** `src/pages/meme/wired-in.astro`
```astro
---
import BaseLayout from '@/layouts/BaseLayout.astro';
---

<BaseLayout title="Wired In">
  <div>
    <iframe
      class="h-screen w-full"
      width="100%"
      height="100vh"
      src="https://www.youtube-nocookie.com/embed/PRBSKaTDrqQ?si=5jlDUh7aXIYev7Ty&controls=0"
      title="YouTube video player"
      allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
      allowfullscreen
    ></iframe>
  </div>
</BaseLayout>
```

---

## Sitemap Integration

Install and configure `@astrojs/sitemap` to generate sitemaps automatically.

### Installation

```bash
pnpm astro add sitemap
```

### Configuration

**astro.config.mjs:**
```javascript
import { defineConfig } from 'astro/config';
import sitemap from '@astrojs/sitemap';

export default defineConfig({
  site: 'https://www.rivet.dev',
  integrations: [
    sitemap({
      // Filter out pages you don't want in sitemap
      filter: (page) => !page.includes('/api/') && !page.includes('/internal/'),

      // Add custom pages not generated by Astro
      customPages: [
        'https://www.rivet.dev/changelog.json',
      ],

      // Split sitemap if > 45000 entries
      entryLimit: 45000,

      // Optional: set changefreq/priority per-page
      serialize: (item) => {
        // Docs pages change more frequently
        if (item.url.includes('/docs/')) {
          item.changefreq = 'weekly';
          item.priority = 0.8;
        }
        return item;
      },
    }),
  ],
});
```

### Output

The integration generates:
- `sitemap-index.xml` - Index file pointing to individual sitemaps
- `sitemap-0.xml`, `sitemap-1.xml`, etc. - Actual sitemap files

These are automatically generated during `astro build`.

---

## llms.txt Generation

The current Next.js site generates `llms.txt` and `llms-full.txt` via a prebuild script. We need to replicate this in Astro.

### Current Implementation

The existing `scripts/generateMarkdownAndLlms.ts` generates markdown files for AI consumption. This script can be adapted for Astro.

### Astro Approach

**Option 1: Keep existing prebuild script**

Continue using the TypeScript script during prebuild:

```json
{
  "scripts": {
    "prebuild": "tsx scripts/generateLlmsTxt.ts",
    "build": "astro build"
  }
}
```

**Option 2: Use Astro endpoint for dynamic generation**

Create `src/pages/llms.txt.ts`:
```typescript
import type { APIRoute } from 'astro';
import { getCollection } from 'astro:content';

export const GET: APIRoute = async () => {
  const docs = await getCollection('docs');
  const posts = await getCollection('posts');

  let content = `# Rivet Documentation\n\n`;
  content += `> Rivet is a platform for building stateful serverless applications.\n\n`;
  content += `## Documentation\n\n`;

  for (const doc of docs) {
    content += `### ${doc.id}\n`;
    // Add rendered content or summary
  }

  return new Response(content, {
    headers: {
      'Content-Type': 'text/plain; charset=utf-8',
    },
  });
};
```

**Option 3: Static file generation during build**

Create an Astro page that outputs as `.txt`:

`src/pages/llms.txt.astro`:
```astro
---
import { getCollection, render } from 'astro:content';

const docs = await getCollection('docs');

let content = `# Rivet Documentation

> Rivet is a platform for building stateful serverless applications.

## Docs

`;

for (const doc of docs) {
  const { headings } = await render(doc);
  const title = headings.find(h => h.depth === 1)?.text || doc.id;
  content += `### ${title}\n`;
  content += `URL: /docs/${doc.id}/\n\n`;
}
---
{content}
```

### Recommended Approach

Keep the existing `generateMarkdownAndLlms.ts` script but adapt it to read from Astro's content directory structure. The script already handles:
- Aggregating all MDX content
- Stripping unnecessary formatting
- Generating both summary (`llms.txt`) and full (`llms-full.txt`) versions

Add to `package.json`:
```json
{
  "scripts": {
    "gen:llms": "tsx scripts/generateLlmsTxt.ts",
    "prebuild": "pnpm gen:navigation && pnpm gen:llms",
    "build": "pnpm prebuild && astro build"
  }
}
```

---

## Prebuild Scripts Migration

The current site uses prebuild scripts that need to be adapted:

### Navigation Generation (`scripts/generateNavigation.ts`)

**Current behavior:**
- Reads all MDX files from `src/content/` and `src/app/(legacy)/blog/`
- Extracts title and description from each file
- Writes to `src/generated/routes.json`

**Astro adaptation:**

In Astro, content collections provide this metadata automatically via `getCollection()`. You may not need this script at all.

However, if you need a JSON file for search or other purposes, adapt the script:

```typescript
// scripts/generateNavigation.ts (adapted for Astro)
import glob from 'fast-glob';
import { readFile, writeFile, mkdir } from 'node:fs/promises';

async function generateNavigation() {
  const pages: Record<string, { title: string; description?: string }> = {};

  const mdxFiles = await glob(['src/content/**/*.mdx'], { cwd: '.' });

  for (const filename of mdxFiles) {
    const content = await readFile(filename, 'utf-8');

    // Extract title from first h1 heading
    const titleMatch = content.match(/^#\s+(.+)$/m);
    const title = titleMatch ? titleMatch[1] : '';

    // Extract description from first paragraph after h1
    const descMatch = content.match(/^#\s+.+\n+([^#\n].+)$/m);
    const description = descMatch ? descMatch[1] : undefined;

    // Build href from filename
    const href = '/' + filename
      .replace('src/content/', '')
      .replace(/\/index\.mdx$/, '')
      .replace(/\.mdx$/, '');

    pages[href] = { title, description };
  }

  await mkdir('./src/generated', { recursive: true });
  await writeFile('./src/generated/routes.json', JSON.stringify({ pages }, null, 2));
  console.log(`Generated ${Object.keys(pages).length} pages`);
}

generateNavigation();
```

### Markdown/LLMs Generation

Keep `scripts/generateMarkdownAndLlms.ts` largely unchanged, just update paths:

- Source: `src/content/docs` (same location)
- Output: `public/llms.txt` and `public/llms-full.txt`

---

## Railway Deployment (Docker + Caddy)

The site is deployed as a static build served by Caddy, matching the current Next.js deployment pattern.

### Dockerfile

Create `website-astro/Dockerfile`:

```dockerfile
# Website Dockerfile
# Multi-stage build: Node.js for building, Caddy for serving

# =============================================================================
# Stage 1: Build
# =============================================================================
FROM node:22-alpine AS builder

# Install git and git-lfs for fetching LFS files
RUN apk add --no-cache git git-lfs

# Install pnpm
RUN corepack enable && corepack prepare pnpm@latest --activate

WORKDIR /app

# Copy workspace configuration files
COPY pnpm-workspace.yaml package.json pnpm-lock.yaml ./

# Copy all workspace packages that website depends on
COPY website-astro/ website-astro/
COPY frontend/packages/components/ frontend/packages/components/
COPY frontend/packages/icons/ frontend/packages/icons/
COPY frontend/packages/example-registry/ frontend/packages/example-registry/
COPY examples/ examples/

# Fetch LFS files if needed
COPY scripts/docker/fetch-lfs.sh /tmp/fetch-lfs.sh
RUN chmod +x /tmp/fetch-lfs.sh && /tmp/fetch-lfs.sh

# Arguments for build
ARG FONTAWESOME_PACKAGE_TOKEN=""
ENV FONTAWESOME_PACKAGE_TOKEN=${FONTAWESOME_PACKAGE_TOKEN}

# Install dependencies
RUN --mount=type=cache,id=pnpm-store,target=/pnpm/store \
    pnpm install --frozen-lockfile

# Build arguments for PUBLIC_* environment variables
ARG PUBLIC_SITE_URL="https://rivet.gg"
ARG PUBLIC_POSTHOG_KEY=""
ARG PUBLIC_POSTHOG_HOST=""
ARG PUBLIC_TYPESENSE_HOST=""
ARG PUBLIC_TYPESENSE_API_KEY=""

# Set environment variables for build
ENV PUBLIC_SITE_URL=${PUBLIC_SITE_URL}
ENV PUBLIC_POSTHOG_KEY=${PUBLIC_POSTHOG_KEY}
ENV PUBLIC_POSTHOG_HOST=${PUBLIC_POSTHOG_HOST}
ENV PUBLIC_TYPESENSE_HOST=${PUBLIC_TYPESENSE_HOST}
ENV PUBLIC_TYPESENSE_API_KEY=${PUBLIC_TYPESENSE_API_KEY}

WORKDIR /app/website-astro

# Build the website (static output to 'dist' directory)
RUN pnpm run build

# =============================================================================
# Stage 2: Serve with Caddy
# =============================================================================
FROM caddy:alpine

# Copy Caddyfile configuration
COPY website-astro/Caddyfile /etc/caddy/Caddyfile

# Copy built files from builder stage
COPY --from=builder /app/website-astro/dist /srv

# Default port
ENV PORT=80

CMD ["caddy", "run", "--config", "/etc/caddy/Caddyfile"]
```

### Caddyfile

Create `website-astro/Caddyfile` (copy from `website/Caddyfile` with minor adjustments):

```caddyfile
{
	admin off
}

:{$PORT:80} {
	root * /srv

	# Gzip compression
	encode gzip

	# Security headers
	header {
		X-Frame-Options "SAMEORIGIN"
		X-Content-Type-Options "nosniff"
		X-XSS-Protection "1; mode=block"
	}

	# CORS for *.rivet.dev subdomains to fetch changelog.json
	@cors_preflight {
		path /changelog.json
		method OPTIONS
		header_regexp Origin ^https://.*\.rivet\.dev$
	}
	handle @cors_preflight {
		header Access-Control-Allow-Origin {header.Origin}
		header Access-Control-Allow-Methods "GET, OPTIONS"
		header Access-Control-Allow-Headers "Content-Type"
		respond 204
	}
	@cors_json {
		path /changelog.json
		header_regexp Origin ^https://.*\.rivet\.dev$
	}
	header @cors_json Access-Control-Allow-Origin {header.Origin}

	# Cache static assets aggressively (Astro outputs to _astro/)
	@static {
		path /_astro/* *.js *.css *.png *.jpg *.jpeg *.gif *.ico *.svg *.woff *.woff2 *.ttf *.eot
	}
	header @static Cache-Control "public, max-age=31536000, immutable"

	# Don't cache HTML files
	@html {
		path *.html
	}
	header @html Cache-Control "no-store, no-cache, must-revalidate"

	# Health check endpoint
	handle /health {
		respond "healthy" 200
	}

	# Main site handler - Astro static export with directory index
	handle {
		try_files {path} {path}/ {path}/index.html
		file_server
	}

	# Custom 404 page
	handle_errors {
		rewrite * /404.html
		file_server
	}
}
```

### astro.config.mjs for Static Build

```javascript
import { defineConfig } from 'astro/config';
import mdx from '@astrojs/mdx';
import react from '@astrojs/react';
import tailwind from '@astrojs/tailwind';
import sitemap from '@astrojs/sitemap';

export default defineConfig({
  site: 'https://www.rivet.dev',
  output: 'static',  // REQUIRED: No SSR allowed
  trailingSlash: 'always',
  build: {
    assets: '_astro',
    format: 'directory',  // Generates /page/index.html instead of /page.html
  },
  integrations: [
    mdx(),
    react(),
    tailwind(),
    sitemap(),
  ],
});
```

### Environment Variables

Astro uses `PUBLIC_` prefix (not `NEXT_PUBLIC_`):

| Next.js | Astro |
|---------|-------|
| `NEXT_PUBLIC_SITE_URL` | `PUBLIC_SITE_URL` |
| `NEXT_PUBLIC_POSTHOG_KEY` | `PUBLIC_POSTHOG_KEY` |
| `NEXT_PUBLIC_POSTHOG_HOST` | `PUBLIC_POSTHOG_HOST` |
| `NEXT_PUBLIC_TYPESENSE_HOST` | `PUBLIC_TYPESENSE_HOST` |
| `NEXT_PUBLIC_TYPESENSE_API_KEY` | `PUBLIC_TYPESENSE_API_KEY` |

Access in code:
```typescript
const siteUrl = import.meta.env.PUBLIC_SITE_URL;
```

### Railway Configuration

**Option 1: Dockerfile (Recommended)**

Railway will automatically detect and use the Dockerfile.

**Option 2: railway.json**

```json
{
  "build": {
    "builder": "DOCKERFILE",
    "dockerfilePath": "website-astro/Dockerfile"
  }
}
```

### Deployment Checklist

- [ ] Verify `output: 'static'` in astro.config.mjs
- [ ] Create Dockerfile with Caddy serving
- [ ] Create Caddyfile with proper routing
- [ ] Test build locally: `pnpm build && npx serve dist`
- [ ] Test Docker build: `docker build -t website-astro .`
- [ ] Push to GitHub
- [ ] Connect to Railway
- [ ] Configure environment variables
- [ ] Set custom domain
- [ ] Verify all routes return 200

---

## Copy Commands

Run these commands to set up the initial file structure:

```bash
# Create directory structure
mkdir -p website-astro/src/{pages,content,components,data,lib,layouts,styles}
mkdir -p website-astro/src/pages/{docs,guides,learn,blog,changelog,templates}
mkdir -p website-astro/public

# Copy content (MDX files)
cp -r website/src/content/docs website-astro/src/content/
cp -r website/src/content/guides website-astro/src/content/
cp -r website/src/content/learn website-astro/src/content/

# Copy blog posts to content folder
cp -r website/src/posts website-astro/src/content/

# Copy components (will need React → Astro conversion for some)
cp -r website/src/components website-astro/src/

# Copy data files (no changes needed)
cp -r website/src/data website-astro/src/

# Copy lib files (some modifications needed)
cp -r website/src/lib website-astro/src/

# Copy MDX configuration (reusable plugins)
cp -r website/src/mdx website-astro/src/

# Copy sitemap/navigation config
cp -r website/src/sitemap website-astro/src/

# Copy authors directory
cp -r website/src/authors website-astro/src/

# Copy public assets
cp -r website/public/* website-astro/public/

# Copy Tailwind config (will need minor adjustments)
cp website/tailwind.config.ts website-astro/tailwind.config.mjs
cp website/postcss.config.js website-astro/postcss.config.cjs

# Copy TypeScript config
cp website/tsconfig.json website-astro/tsconfig.json
```

---

## Example Migrations

### Example A: Docs Page with generateStaticParams

**Before (Next.js) - `src/app/(v2)/[section]/[[...page]]/page.tsx`:**
```typescript
import fs from "node:fs/promises";
import path from "node:path";
import type { Metadata } from "next";
import { notFound } from "next/navigation";

export const dynamicParams = false;

export async function generateStaticParams() {
  const staticParams: Param[] = [];
  for (const section of VALID_SECTIONS) {
    const dir = path.join(process.cwd(), "src", "content", section);
    const dirs = await fs.readdir(dir, { recursive: true });
    const files = dirs.filter((file) => file.endsWith(".mdx"));
    const sectionParams = files.map((file) => createParamsForFile(section, file));
    staticParams.push(...sectionParams);
  }
  return staticParams;
}

export async function generateMetadata({ params }): Promise<Metadata> {
  const { section, page } = await params;
  const path = buildPathComponents(section, page);
  const { component: { title, description } } = await loadContent(path);
  return {
    title: `${title} - Rivet`,
    description,
    alternates: { canonical: `https://www.rivet.dev${buildFullPath(path)}/` },
  };
}

export default async function CatchAllCorePage({ params }) {
  const { section, page } = await params;
  if (!VALID_SECTIONS.includes(section)) return notFound();

  const path = buildPathComponents(section, page);
  const { component: { default: Content, tableOfContents, title } } = await loadContent(path);

  return (
    <>
      <DocsNavigation sidebar={foundTab?.tab.sidebar} />
      <Prose as="article">
        <Content />
      </Prose>
      <DocsTableOfContents tableOfContents={tableOfContents} />
    </>
  );
}
```

**After (Astro) - `src/pages/docs/[...slug].astro`:**
```astro
---
import { getCollection, render } from 'astro:content';
import DocsLayout from '@/layouts/DocsLayout.astro';
import DocsNavigation from '@/components/DocsNavigation.astro';
import DocsTableOfContents from '@/components/DocsTableOfContents.astro';
import Prose from '@/components/Prose.astro';
import * as mdxComponents from '@/components/mdx';
import { sitemap } from '@/sitemap/mod';
import { findActiveTab } from '@/lib/sitemap';

export async function getStaticPaths() {
  const docs = await getCollection('docs');
  return docs.map((entry) => ({
    params: { slug: entry.id },
    props: { entry },
  }));
}

interface Props {
  entry: Awaited<ReturnType<typeof getCollection<'docs'>>>[number];
}

const { entry } = Astro.props;
const { Content, headings } = await render(entry);

// Build table of contents from headings
const tableOfContents = headings
  .filter(h => h.depth === 2 || h.depth === 3)
  .reduce((acc, h) => {
    if (h.depth === 2) {
      acc.push({ title: h.text, id: h.slug, children: [] });
    } else if (acc.length > 0) {
      acc[acc.length - 1].children.push({ title: h.text, id: h.slug, children: [] });
    }
    return acc;
  }, [] as Array<{ title: string; id: string; children: Array<{ title: string; id: string; children: never[] }> }>);

// Get title from first h1 or frontmatter
const title = entry.data.title || headings.find(h => h.depth === 1)?.text || 'Documentation';
const description = entry.data.description || '';
const fullPath = `/docs/${entry.id}/`;
const foundTab = findActiveTab(fullPath, sitemap);
const canonicalUrl = `https://www.rivet.dev${fullPath}`;
---

<DocsLayout
  title={`${title} - Rivet`}
  description={description}
  canonicalUrl={canonicalUrl}
>
  <aside class="hidden lg:block border-r" slot="sidebar">
    {foundTab?.tab.sidebar && <DocsNavigation sidebar={foundTab.tab.sidebar} />}
  </aside>

  <main class="w-full py-8 px-8">
    <Prose as="article" class="max-w-prose mx-auto">
      <Content components={mdxComponents} />
    </Prose>
  </main>

  <aside class="hidden xl:block w-64" slot="toc">
    <DocsTableOfContents tableOfContents={tableOfContents} />
  </aside>
</DocsLayout>
```

### Example B: Blog Post with Image and Metadata

**Before (Next.js) - `src/app/(v2)/(blog)/blog/[...slug]/page.tsx`:**
```typescript
import { generateArticlesPageParams, loadArticle } from "@/lib/article";
import type { Metadata } from "next";
import Image from "next/image";

export async function generateMetadata({ params }): Promise<Metadata> {
  const { slug } = await params;
  const { description, title, author, published, tags, category, image } =
    await loadArticle(slug.join("/"));

  return {
    title,
    description,
    authors: [{ name: author.name, url: author.socials?.twitter || "" }],
    keywords: tags,
    openGraph: {
      title,
      description,
      type: "article",
      publishedTime: new Date(published).toISOString(),
      images: [{ url: image.src, width: image.width, height: image.height }],
    },
  };
}

export default async function BlogPage({ params }) {
  const { slug } = await params;
  const { Content, title, tableOfContents, author, published, category, image } =
    await loadArticle(slug.join("/"));

  return (
    <article>
      <Image {...image} alt="Promo Image" />
      <Content />
    </article>
  );
}

export function generateStaticParams() {
  return generateArticlesPageParams();
}
```

**After (Astro) - `src/pages/blog/[...slug].astro`:**
```astro
---
import { getCollection, render } from 'astro:content';
import { Image } from 'astro:assets';
import BlogLayout from '@/layouts/BlogLayout.astro';
import { AUTHORS, CATEGORIES } from '@/lib/article';
import * as mdxComponents from '@/components/mdx';

export async function getStaticPaths() {
  const posts = await getCollection('posts');

  // Import all post images eagerly
  const images = import.meta.glob<{ default: ImageMetadata }>(
    '/src/content/posts/*/image.{png,jpg,gif}',
    { eager: true }
  );

  return posts.map((entry) => {
    const slug = entry.id.replace(/\/page$/, '');
    const imagePath = Object.keys(images).find(p => p.includes(slug));
    const image = imagePath ? images[imagePath].default : null;

    return {
      params: { slug },
      props: { entry, image },
    };
  });
}

interface Props {
  entry: Awaited<ReturnType<typeof getCollection<'posts'>>>[number];
  image: ImageMetadata | null;
}

const { entry, image } = Astro.props;
const { Content, headings } = await render(entry);

const author = AUTHORS[entry.data.author];
const category = CATEGORIES[entry.data.category];
const title = headings.find(h => h.depth === 1)?.text || 'Blog Post';
const description = entry.data.description || '';
const published = entry.data.published;

// Build table of contents
const tableOfContents = headings
  .filter(h => h.depth === 2 || h.depth === 3)
  .reduce((acc, h) => {
    if (h.depth === 2) acc.push({ title: h.text, id: h.slug, children: [] });
    else if (acc.length > 0) acc[acc.length - 1].children.push({ title: h.text, id: h.slug, children: [] });
    return acc;
  }, [] as any[]);
---

<BlogLayout
  title={title}
  description={description}
  author={author}
  published={published}
  category={category}
  image={image}
  tableOfContents={tableOfContents}
>
  <article>
    {image && (
      <Image
        src={image}
        alt="Promo Image"
        class="rounded-xl border border-white/10"
      />
    )}
    <Content components={mdxComponents} />
  </article>
</BlogLayout>
```

### Example C: Templates Page (Data-driven, not MDX)

**Before (Next.js) - `src/app/(v2)/(marketing)/templates/[slug]/page.tsx`:**
```typescript
import { templates, TECHNOLOGIES, TAGS } from "@/data/templates/shared";
import type { Metadata } from "next";
import { notFound } from "next/navigation";

export async function generateStaticParams() {
  return templates.map((template) => ({ slug: template.name }));
}

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const { slug } = await params;
  const template = templates.find((t) => t.name === slug);
  if (!template) return { title: "Template Not Found - Rivet" };
  return {
    title: `${template.displayName} - Rivet Templates`,
    description: template.description,
    alternates: { canonical: `https://www.rivet.dev/templates/${slug}/` },
  };
}

export default async function Page({ params }: Props) {
  const { slug } = await params;
  const template = templates.find((t) => t.name === slug);
  if (!template) notFound();

  return <TemplateContent template={template} />;
}
```

**After (Astro) - `src/pages/templates/[slug].astro`:**
```astro
---
import { templates, TECHNOLOGIES, TAGS } from '@/data/templates/shared';
import TemplateLayout from '@/layouts/TemplateLayout.astro';
import { Image } from 'astro:assets';

export async function getStaticPaths() {
  return templates.map((template) => ({
    params: { slug: template.name },
    props: { template },
  }));
}

interface Props {
  template: typeof templates[number];
}

const { template } = Astro.props;
const canonicalUrl = `https://www.rivet.dev/templates/${template.name}/`;
const description = template.description.replace(/\[([^\]]+)\]\([^)]+\)/g, '$1');
---

<TemplateLayout
  title={`${template.displayName} - Rivet Templates`}
  description={description}
  canonicalUrl={canonicalUrl}
>
  <main class="min-h-screen w-full max-w-[1500px] mx-auto md:px-8">
    {!template.noFrontend && (
      <div class="relative">
        <Image
          src={`/examples/${template.name}/image.png`}
          alt={template.displayName}
          width={1200}
          height={675}
          class="h-full w-full object-cover"
        />
      </div>
    )}

    <h1 class="text-3xl font-medium tracking-tight text-white md:text-5xl">
      {template.displayName}
    </h1>
    <p class="text-lg text-zinc-400">{description}</p>

    <!-- Technologies -->
    <div class="flex flex-wrap gap-2">
      {template.technologies.map((tech) => {
        const techInfo = TECHNOLOGIES.find((t) => t.name === tech);
        return (
          <span class="inline-flex items-center px-3 py-1.5 rounded-md text-sm bg-white/5 text-zinc-300 border border-white/10">
            {techInfo?.displayName || tech}
          </span>
        );
      })}
    </div>
  </main>
</TemplateLayout>
```

---

## Migration Checklist

### Phase 1: Project Setup
- [ ] Initialize Astro project with `pnpm create astro@latest website-astro`
- [ ] Install integrations: `@astrojs/mdx`, `@astrojs/react`, `@astrojs/tailwind`, `@astrojs/sitemap`
- [ ] Copy configuration files (tailwind, postcss, tsconfig)
- [ ] Configure `astro.config.mjs` with site URL, integrations, and plugins
- [ ] Set up content collections in `content.config.ts`
- [ ] Copy and adapt MDX plugins from `src/mdx/`

### Phase 2: Content Migration
- [ ] Copy MDX content directories (`docs/`, `guides/`, `learn/`)
- [ ] Copy blog posts to `src/content/posts/`
- [ ] Convert blog post front matter from JS exports to YAML frontmatter
- [ ] Update any MDX imports/exports that aren't compatible
- [ ] Verify MDX plugins work with Astro's MDX integration

### Phase 3: Components
- [ ] Create base layout (`BaseLayout.astro`) with `<html>`, `<head>`, `<body>`
- [ ] Create specialized layouts (`DocsLayout.astro`, `BlogLayout.astro`, `LearnLayout.astro`)
- [ ] Keep React components for interactive elements (use `client:load` directive)
- [ ] Create Astro wrappers for MDX components (`src/components/mdx.ts`)
- [ ] Migrate navigation components to Astro

### Phase 4: Pages
- [ ] Create docs catch-all route (`src/pages/docs/[...slug].astro`)
- [ ] Create guides catch-all route (`src/pages/guides/[...slug].astro`)
- [ ] Create learn section routes (`src/pages/learn/[...slug].astro`)
- [ ] Create blog routes with image handling (`src/pages/blog/[...slug].astro`)
- [ ] Create changelog routes (`src/pages/changelog/[...slug].astro`)
- [ ] Create templates routes (`src/pages/templates/[slug].astro`)
- [ ] Create marketing pages (home, pricing, cloud, etc.)
- [ ] Create index pages for blog, changelog, templates listings

### Phase 5: Integrations
- [ ] Configure sitemap generation with filters
- [ ] Set up llms.txt generation (prebuild script or endpoint)
- [ ] Migrate RSS/JSON feed generation
- [ ] Set up image optimization with `astro:assets`

### Phase 6: Testing & Deployment
- [ ] Verify all routes generate correctly with `astro build`
- [ ] Test MDX rendering with all custom components
- [ ] Verify image optimization and lazy loading
- [ ] Test sitemap output
- [ ] Validate llms.txt generation
- [ ] Set up Railway deployment
- [ ] Configure custom domain and HTTPS
- [ ] Test production build

---

## Package.json Scripts

```json
{
  "name": "rivet-site",
  "type": "module",
  "scripts": {
    "dev": "astro dev",
    "build": "pnpm prebuild && astro build",
    "preview": "astro preview",
    "prebuild": "pnpm gen:navigation && pnpm gen:llms",
    "gen:navigation": "tsx scripts/generateNavigation.ts",
    "gen:llms": "tsx scripts/generateLlmsTxt.ts",
    "astro": "astro"
  },
  "dependencies": {
    "astro": "^5.x",
    "@astrojs/mdx": "^4.x",
    "@astrojs/react": "^4.x",
    "@astrojs/tailwind": "^6.x",
    "@astrojs/sitemap": "^3.x",
    "react": "^19.x",
    "react-dom": "^19.x",
    "tailwindcss": "^3.x",
    "shiki": "^3.x",
    "@shikijs/transformers": "^3.x",
    "mdx-annotations": "^0.1.x",
    "remark-gfm": "^4.x",
    "@sindresorhus/slugify": "^3.x"
  }
}
```

---

## References

- [Astro Documentation: Migrating from Next.js](https://docs.astro.build/en/guides/migrate-to-astro/from-nextjs/)
- [Astro Content Collections](https://docs.astro.build/en/guides/content-collections/)
- [Astro MDX Integration](https://docs.astro.build/en/guides/integrations-guide/mdx/)
- [Astro Sitemap Integration](https://docs.astro.build/en/guides/integrations-guide/sitemap/)
- [Astro getStaticPaths](https://docs.astro.build/en/reference/routing-reference/)
- [Astro Deploy to Railway](https://docs.astro.build/en/guides/deploy/railway/)
- [Railway Astro SSR Guide](https://docs.railway.com/guides/astro)
- [Astro Build with AI (llms.txt)](https://docs.astro.build/en/guides/build-with-ai/)
