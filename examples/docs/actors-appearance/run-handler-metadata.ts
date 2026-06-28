import type { RunConfig } from "rivetkit";

type MyOptions = {
  mode?: "safe" | "fast";
};

function myCustomRunHandler(_options: MyOptions): RunConfig {
  const run: RunConfig["run"] = async (_c) => {
    // Your run handler logic...
  };

  return {
    name: "My Custom Handler",
    icon: "bolt",
    run,
  };
}
