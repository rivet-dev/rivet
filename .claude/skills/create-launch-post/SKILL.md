---
name: create-launch-post
description: Create and publish concise Rivet launch or changelog posts, including release research, user-selected painting artwork in the shared Figma launch template, R2 hero upload, MDX bootstrapping, validation, and a draft GitHub PR for manual merge. Use when asked to launch, announce, ship, or prepare a post for a Rivet feature or release.
---

# Create Launch Post

Prepare a complete, deliberately short launch post. Keep the painting choice and final merge as user-owned checkpoints.

## 1. Start a revision and research the release

1. Read the repository `AGENTS.md` files that govern the files in scope.
2. Before editing, run `jj status`, initialize with `jj git init --colocate` if needed, then run `jj new` and `jj describe -m "docs(website): launch <feature>"`.
3. Read the source PR, changed docs, examples, tests, and public API. Treat implemented code and merged docs as authoritative; do not infer syntax from the PR title.
4. Read at least three recent concise posts in `website/src/content/posts/`, preferring `category: changelog` posts for similar features.
5. Resolve the public docs route, GitHub URL, and Discord URL before drafting.

## 2. Create the hero in Figma

1. Ask the user to choose and provide a painting. This is a required creative checkpoint; do not select one silently. Ask for the source or confirm that it is safe to publish.
2. Ask for the shared launch-template Figma node URL if it cannot be discovered from available Figma context. Do not recreate the visual system from memory.
3. Duplicate the latest generic launch frame. Preserve its typography, logo treatment, and 2:1 `2048x1024` layout.
4. Replace the title and painting, then visually verify that the crop, contrast, and text are clean.
5. Export the selected frame as PNG to a temporary path outside the repository.

Do not add `image: true` to the post until the uploaded asset resolves successfully.

## 3. Upload the hero

Use the post folder name as `<slug>` and upload exactly:

```bash
AWS_ACCESS_KEY_ID='op://Engineering/rivet-assets R2 Upload/username' \
AWS_SECRET_ACCESS_KEY='op://Engineering/rivet-assets R2 Upload/password' \
op run -- aws s3 cp "$IMAGE_PATH" \
  "s3://rivet-assets/website/blog/<slug>/image.png" \
  --endpoint-url https://2a94c6a0ced8d35ea63cddc86c2681e7.r2.cloudflarestorage.com
```

Verify `https://assets.rivet.dev/website/blog/<slug>/image.png`, then add `image: true` to frontmatter. Never commit the exported hero to Git and never print resolved credentials.

## 4. Bootstrap the MDX

Run the repository generator:

```bash
pnpm --dir website new-post
```

For a future launch date, bootstrap normally, then rename the generated folder and update `published` to `YYYY-MM-DD`. The final path is:

```text
website/src/content/posts/YYYY-MM-DD-<slug>/page.mdx
```

Use `author: nathan-flurry` unless the user specifies another author, `category: changelog`, and targeted lowercase keywords.

## 5. Write the generic launch format

Keep the body brief and use exactly this order:

1. One short paragraph explaining what shipped and why it matters.
2. `## Show Me The Code` with one small, complete example copied from or checked against the release.
3. `## More Links` with docs, `https://github.com/rivet-dev/rivet`, and `https://rivet.dev/discord`.

Aim for under 200 prose words excluding code. Include all TypeScript imports and referenced definitions. Avoid roadmap claims, implementation detail, a long feature inventory, or repeated calls to action.

## 6. Validate

1. Confirm the folder date, `published`, title, description, keywords, and image slug agree.
2. Check the code against the source PR and current documentation.
3. Run the narrowest available website formatting/type checks, then `pnpm --dir website build` when dependencies and checkout scope allow it.
4. Review `jj diff` and `jj status`; keep unrelated changes out of the revision.
5. Confirm the hero URL returns successfully before leaving `image: true` enabled.

## 7. Open a draft PR and stop before merge

Only push when the user has requested publishing to GitHub. Create an `agent/<description>` bookmark for the revision, push that bookmark, and open a draft PR against `main`. Use a conventional PR title and a short bullet-list body covering the post, artwork, skill changes, and validation.

Return the PR URL and explicitly tell the user that they must review and merge it manually. Never merge the launch PR on the user's behalf.

If the painting, Figma node, asset credentials, or release API is missing, complete the safe independent work and report the exact remaining checkpoint instead of fabricating it.
