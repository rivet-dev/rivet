# Content frontmatter

Required frontmatter schemas for website content.

## Docs (`website/src/content/docs/**/*.mdx`)

Required fields:

- `title` (string)
- `description` (string)
- `skill` (boolean)

## Blog + Changelog (`website/src/content/posts/**/page.mdx`)

Required fields:

- `title` (string)
- `description` (string)
- `author` (enum: `nathan-flurry`, `nicholas-kissel`, `forest-anderson`)
- `published` (date string)
- `category` (enum: `changelog`, `monthly-update`, `launch-week`, `technical`, `guide`, `frogs`)

Optional fields:

- `keywords` (string array)
- `image` (`true` or `{ format: string }`) — hero image flag. Presence means the post has a hero image in R2 at `website/blog/{post-slug}/image.{format}`; use `image: true` for the default `image.png` or `image: { format: "gif" }` for another extension. The URL is derived from the slug and dimensions are fixed (2:1), so do not write `src`, `width`, or `height`. Resolved by `website/src/lib/postImage.ts`.
- `unpublished` (boolean)
