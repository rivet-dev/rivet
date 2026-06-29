import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

interface Item {
  type: string;
  data: string;
}

const processor = actor({
  state: {},
  actions: {
    process: (c, item: Item) => ({ processed: true, item })
  }
});

const registry = setup({ use: { processor } });
const client = createClient<typeof registry>("http://localhost:6420");

// Process items in parallel
const items: Item[] = [
  { type: "typeA", data: "data1" },
  { type: "typeB", data: "data2" }
];

const results = await Promise.all(
  items.map(item => client.processor.getOrCreate([item.type]).process(item))
);
