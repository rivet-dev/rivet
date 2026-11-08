import { actor } from "rivetkit";
import { AgentOs } from "@rivet-dev/agentos-core";

export const sandbox = actor({
  createVars: async () => ({ vm: await AgentOs.create() }),
  onSleep: async (c) => {
    await c.vars.vm.dispose();
  },
  onDestroy: async (c) => {
    await c.vars.vm.dispose();
  },

  actions: {
    // docs:start shell
    exec: async (c, command: string) => {
      const result = await c.vars.vm.process.exec(command);
      return result.stdout;
    },
    // docs:end shell

    // docs:start files
    writeFile: async (c, path: string, contents: string) => {
      await c.vars.vm.filesystem.writeFile(path, contents);
    },
    readFile: async (c, path: string) => {
      const bytes = await c.vars.vm.filesystem.readFile(path);
      return new TextDecoder().decode(bytes);
    },
    // docs:end files

    // docs:start python
    python: async (c, code: string) => {
      const result = await c.vars.vm.python.execute(code);
      return result.stdout;
    },
    // docs:end python
  },
});
