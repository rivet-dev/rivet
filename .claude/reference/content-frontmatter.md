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
