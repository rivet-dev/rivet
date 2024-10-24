import fs from 'fs';

let backendPath = '../rivet';

let errorsPath = 'src/docs/general/errors';

export async function generateErrors() {
  let errorPages = [];
  fs.rmSync(errorsPath, { recursive: true, force: true });

  processErrorDir(`${backendPath}/errors`, errorPages);

  function processErrorDir(inputPath, pages) {
    console.log(`Processing dir ${inputPath}`);

    for (const dirEntry of fs.readdirSync(inputPath)) {
      let inputPathEntry = `${inputPath}/${dirEntry}`;

      let stat = fs.statSync(inputPathEntry);

      if (stat.isFile && dirEntry.endsWith('.md')) {
        console.log(`Processing file ${inputPath}`);

        let errorDoc = fs.readFileSync(inputPathEntry, 'utf8');

        // Read metadata
        let title = errorDoc.match(/^#\s+(.*)$/m)[1];
        if (!title) throw `Missing title: ${inputPathEntry}`;
        let name = errorDoc.match(/^name\s*=\s*"([\w_]+)"\s*$/m)[1];
        if (!name) throw `Missing name: ${inputPathEntry}`;
        let httpStatus = parseInt(errorDoc.match(/^http_status\s*=\s*(\d+)\s*$/m)[1]);
        if (httpStatus >= 500 && httpStatus < 600) {
          continue;
        }
        let isDeprecated = errorDoc.match(/^deprecated\s*=\s*true\s*$/m);
        let isExperimental = errorDoc.match(/^experimental\s*=\s*true\s*$/m);

        // Strip error doc
        errorDoc = errorDoc.replace(/---.*---\s+#[^\n]+\s+/gs, '');
        errorDoc = errorDoc.replace(/<!--(.*?)-->/gs, '');
        errorDoc = `## ${title}\n\n<Summary>{\`${name}\`}</Summary>\n\n${errorDoc}`;

        // Add to index of error pages if not deprecated
        if (!isDeprecated && !isExperimental) {
          pages.push({ title: title, doc: errorDoc });
        }
      } else if (stat.isDirectory) {
        processErrorDir(inputPathEntry, pages);

        // TODO: For nested pages
        // let subPages = [];
        // processErrorDir(inputPathEntry, outputPathEntry, subPages);
        // if (subPages.length > 0) {
        //   pages.unshift({
        //     group: dirEntry,
        //     pages: subPages
        //   });
        // }
      }
    }

    pages.sort((a, b) => a.title.localeCompare(b.title));
  }

  fs.writeFileSync(
    'src/docs/general/errors.mdx',
    `# Errors \n${errorPages.map(({ doc }) => doc).join('\n\n')}`
  );

  // fs.writeFileSync('src/generated/errorPages.json', JSON.stringify(errorPages, null, 2));
}

generateErrors();
