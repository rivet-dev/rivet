import { schema, Query } from "./schema.gen.ts";

export const dbEntry = schema.table("db_entry", {
  id: Query.uuid("id").primaryKey().defaultRandom(),
  test2: Query.text("test2"),
  test3: Query.text("test3"),
  test4: Query.boolean("test5"),
});

export const table2 = schema.table("table8", {
  id: Query.uuid("id").primaryKey().defaultRandom(),
  hello: Query.text("hello2"),
});

