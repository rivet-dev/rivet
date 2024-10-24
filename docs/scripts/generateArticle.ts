import { input, select } from "@inquirer/prompts";
import fs from "node:fs/promises";
import path from "node:path";
import { AUTHORS, CATEGORIES } from "../src/lib/article";
import dedent from "dedent";

const BASE_PATH = path.join(__dirname, "../src/posts");

async function run() {
  const title = await input({ message: "Title", required: true });
  const description = await input({ message: "Description" });
  const date = await input({
    message: "Date (YYYY-MM-DD)",
    required: true,
    default: new Date().toISOString().split("T")[0],
    validate: (value) => {
      if (!/^\d{4}-\d{2}-\d{2}$/.test(value)) {
        return "Invalid date format. Please use YYYY-MM-DD.";
      }
      return true;
    },
  });

  const category = await select({
    message: "Category",
    choices: Object.entries(CATEGORIES).map(([key, value]) => ({
      value: key,
      name: value.name,
    })),
  });

  const tags = await input({ message: "Tags (comma separated)" });

  const author = await select({
    message: "Author",
    choices: Object.entries(AUTHORS).map(([key, value]) => ({
      value: key,
      name: value.name,
    })),
  });

  const slug = [date, title.replace(/[^a-zA-Z0-9]+/g, "-").toLowerCase()].join("-");

  await fs.mkdir(path.join(BASE_PATH, slug), {
    recursive: true,
  });

  const articlePath = path.join(BASE_PATH, slug, "page.mdx");

  await fs.writeFile(
    articlePath,
    dedent`export const author = ${JSON.stringify(author)}
    export const published = ${JSON.stringify(date)}
    export const category = ${JSON.stringify(category)}
    export const keywords = ${
      JSON.stringify(tags.split(",").map((tag) => tag.trim()) || [])
    }
    
    # ${title}
    
    ${description}



    
    `,
  );

  console.log(
    `Article created at ${articlePath}`,
  );

  console.log("⚠️ Don't forget to add an image to the article folder!");
}

run().catch(() => {
  process.exit(1);
});
