import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import journal from './meta/_journal.json' with { type: 'json' };

const __dirname = dirname(fileURLToPath(import.meta.url));
const m0000 = readFileSync(join(__dirname, '0000_left_wrecking_crew.sql'), 'utf-8');

export default {
  journal,
  migrations: {
    m0000
  }
}
