import { writeFile, readFile } from 'fs/promises';
import { remark } from 'remark';
import glob from 'fast-glob';
import apiPages from '../src/generated/apiPages.json' assert { type: 'json' };
import engineStyles from '../src/lib/engineStyles.json' assert { type: 'json' };
import { slugifyWithCounter } from '@sindresorhus/slugify';
import { visit } from 'unist-util-visit';
import { toString } from 'mdast-util-to-string';

export async function generateNavigation() {
  // Process all pages
  let pages = {};
  let mdxFileNames = await glob(['app/(legacy)/blog/**/*.mdx', 'docs/**/*.mdx'], {
    cwd: 'src'
  });
  for (let filename of mdxFileNames) {
    let href =
      '/' +
      filename
        .replace(/\/index\.mdx$/, '')
        .replace(/\.mdx$/, '')
        .replace(/^pages\//, '')
        .replace(/^app\//, '')
        .replace(/\/page$/, '')
        .replace('(technical)/', '')
        .replace('(posts)/', '')
        .replace('(legacy)/', '');

    pages[href] = await processPage({ path: filename });
  }

  await writeFile('./src/generated/routes.json', JSON.stringify({ pages }, null, 2), 'utf8');
}

async function processPage({ path }) {
  let md = await readFile(`src/${path}`);

  let ast = remark().parse(md);

  // Title
  let firstHeadingIndex = ast.children.findIndex(node => node.type === 'heading');
  let firstHeading = ast.children[firstHeadingIndex];
  let title = '';
  if (firstHeading) {
    title = firstHeading.children[0].value;
  }

  // Description
  let description = null;
  if (firstHeadingIndex !== -1) {
    for (let i = firstHeadingIndex + 1; i < ast.children.length; i++) {
      let node = ast.children[i];
      if (node.type === 'paragraph') {
        // Stop iterating once we reach a paragraph. Means there's a description.
        description = node.children[0].value;
        break;
      } else if (node.type === 'heading') {
        // Stop iterating once we reach a new heading. Means there's no description.
        break;
      }
    }
  }

  return {
    title,
    description
  };
}

await generateNavigation();
