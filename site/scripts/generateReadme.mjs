#!/usr/bin/env node

import { readFileSync, writeFileSync, existsSync } from 'fs';
import { join } from 'path';
import { EXAMPLE_METADATA } from './examplesData.mjs';

const RIVET_TEMPLATE_PATH = join(process.cwd(), '..', 'README.rivet.tpl.md');
const RIVET_OUTPUT_PATH = join(process.cwd(), '..', 'README.md');

const RIVETKIT_TEMPLATE_PATH = join(process.cwd(), '..', 'README.rivetkit.tpl.md');

const RIVET_QUICKSTART = `Get started with Rivet by following a quickstart guide:

- [Node.js & Bun](https://www.rivet.gg/docs/actors/quickstart/backend/)
- [React](https://www.rivet.gg/docs/actors/quickstart/react/)
`

// Content chunks
const RIVET_FEATURES_CONTENT = `## Features

Rivet Actors are a primitive of RivetKit provide everything you need to build fast, scalable, and real-time applications without the complexity. Rivet Engine is the core of self-hosting and is used for orchestrating actors at scale.

- **Long-Lived, Stateful Compute**: Like AWS Lambda but with memory and no timeouts
- **Blazing-Fast Reads & Writes**: State stored on same machine as compute  
- **Realtime, Made Simple**: Built-in WebSockets and SSE support
- **Store Data Near Your Users**: Deploy to the edge for low-latency access
- **Infinitely Scalable**: Auto-scale from zero to millions without configuration
- **Fault Tolerant**: Automatic error handling and recovery built-in

## BYO DB (Bring Your Own Database)
The Rivet Engine supports:

- **PostgreSQL**: For production deployments
- **FoundationDB**: For enterprise-scale distributed systems
- **Filesystem**: For single-node deployments`;

const RIVETKIT_FEATURES_CONTENT = `## Features

RivetKit provides everything you need to build fast, scalable, and real-time applications without the complexity.

- **Long-Lived, Stateful Compute**: Like AWS Lambda but with memory and no timeouts
- **Blazing-Fast Reads & Writes**: State stored on same machine as compute  
- **Realtime, Made Simple**: Built-in WebSockets and SSE support
- **Store Data Near Your Users**: Deploy to the edge for low-latency access
- **Infinitely Scalable**: Auto-scale from zero to millions without configuration
- **Fault Tolerant**: Automatic error handling and recovery built-in`;

const RIVET_COMMUNITY_CONTENT = `## Community & Support

Join thousands of developers building with Rivet Actors today:

- [Discord](https://rivet.gg/discord) - Chat with the community
- [X/Twitter](https://x.com/rivet_gg) - Follow for updates
- [Bluesky](https://bsky.app/profile/rivet.gg) - Follow for updates
- [GitHub Discussions](https://github.com/rivet-gg/rivetkit/discussions) - Ask questions and share ideas
- [GitHub Issues](https://github.com/rivet-gg/rivetkit/issues) - Report bugs and request features
- [Talk to an engineer](https://rivet.gg/talk-to-an-engineer) - Discuss your technical needs, current stack, and how Rivet can help with your infrastructure challenges`;

const RIVETKIT_COMMUNITY_CONTENT = `## Community & Support

Join thousands of developers building with RivetKit today:

- [Discord](https://rivet.gg/discord) - Chat with the community
- [X/Twitter](https://x.com/rivet_gg) - Follow for updates
- [Bluesky](https://bsky.app/profile/rivet.gg) - Follow for updates
- [GitHub Discussions](https://github.com/rivet-gg/rivetkit/discussions) - Ask questions and share ideas
- [GitHub Issues](https://github.com/rivet-gg/rivetkit/issues) - Report bugs and request features
- [Talk to an engineer](https://rivet.gg/talk-to-an-engineer) - Discuss your technical needs, current stack, and how Rivet can help with your infrastructure challenges`;

const LICENSE_CONTENT = `## License

[Apache 2.0](LICENSE)`;

const RIVET_HEADER = `<!-- 
THIS FILE IS AUTO-GENERATED. DO NOT EDIT DIRECTLY.
To update this README, run: npm run gen:readme
Generated from: site/scripts/generateReadme.mjs
-->

`;

const RIVETKIT_HEADER = `<!-- 
THIS FILE IS AUTO-GENERATED. DO NOT EDIT DIRECTLY.
To update this README, run: npm run gen:readme in the github.com/rivet-gg/rivet repository
Generated from: github.com/rivet-gg/rivet/site/scripts/generateReadme.mjs
-->

`;

function generateExamplesList() {
  return Object.entries(EXAMPLE_METADATA)
    .map(([id, example]) => {
      const githubUrl = `https://github.com/rivet-gg/rivetkit/tree/main/examples/${id}`;
      const stackblitzUrl = `https://stackblitz.com/github/rivet-gg/rivetkit/tree/main/examples/${id}`;
      return `- ${example.title} — [GitHub](${githubUrl}) · [StackBlitz](${stackblitzUrl})`;
    })
    .join('\n');
}

function generateRivetReadme() {
  console.log('Generating Rivet README.md...');
  
  // Generate examples list
  const examplesList = generateExamplesList();
  
  // Read template
  const template = readFileSync(RIVET_TEMPLATE_PATH, 'utf-8');
  
  // Replace placeholders
  let content = template.replace('__EXAMPLES__', examplesList);
  content = content.replace('__QUICKSTART__', RIVET_QUICKSTART);
  content = content.replace('__FEATURES__', RIVET_FEATURES_CONTENT);
  content = content.replace('__COMMUNITY__', RIVET_COMMUNITY_CONTENT);
  content = content.replace('__LICENSE__', LICENSE_CONTENT);
  
  const readmeContent = RIVET_HEADER + content;

  writeFileSync(RIVET_OUTPUT_PATH, readmeContent);
  console.log(`✓ Generated Rivet README.md at ${RIVET_OUTPUT_PATH}`);
}

function generateRivetKitReadme() {
  const rivetKitPath = process.env.RIVETKIT_PATH;
  
  if (!rivetKitPath) {
    console.warn('⚠️  Warning: RIVETKIT_PATH environment variable is not set');
    console.warn('   Skipping RivetKit README generation');
    console.warn('   Set RIVETKIT_PATH to the path of your rivetkit repository');
    return;
  }
  
  // Check if RivetKit path exists
  if (!existsSync(rivetKitPath)) {
    console.error(`❌ Error: RIVETKIT_PATH directory does not exist: ${rivetKitPath}`);
    return;
  }
  
  console.log('Generating RivetKit README.md...');
  
  const rivetKitOutputPath = join(rivetKitPath, 'README.md');
  
  // Generate examples list
  const examplesList = generateExamplesList();
  
  // Read template
  const template = readFileSync(RIVETKIT_TEMPLATE_PATH, 'utf-8');
  
  // Replace placeholders
  let content = template.replace('__EXAMPLES__', examplesList);
  content = content.replace('__QUICKSTART__', RIVET_QUICKSTART);
  content = content.replace('__FEATURES__', RIVETKIT_FEATURES_CONTENT);
  content = content.replace('__COMMUNITY__', RIVETKIT_COMMUNITY_CONTENT);
  content = content.replace('__LICENSE__', LICENSE_CONTENT);
  
  const readmeContent = RIVETKIT_HEADER + content;

  writeFileSync(rivetKitOutputPath, readmeContent);
  console.log(`✓ Generated RivetKit README.md at ${rivetKitOutputPath}`);
}

function main() {
  // Always generate Rivet README
  generateRivetReadme();
  
  // Generate RivetKit README if RIVETKIT_PATH is set
  generateRivetKitReadme();
}

main();
