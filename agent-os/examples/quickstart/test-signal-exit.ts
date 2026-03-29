import { AgentOs } from "@rivet-dev/agent-os-core";
import common from "@rivet-dev/agent-os-common";
import pi from "@rivet-dev/agent-os-pi";
import { resolve } from "path";

async function main() {
  const vm = await AgentOs.create({ 
    software: [common, pi],
    moduleAccessCwd: resolve(import.meta.dirname, "node_modules/@mariozechner/pi-coding-agent"),
  });

  // Write test script to VM filesystem
  await vm.writeFile("/tmp/test.cjs", `
    try {
      var onExit = require('signal-exit');
      console.log('signal-exit type:', typeof onExit);
      console.log('signal-exit value:', String(onExit).slice(0, 100));
    } catch(e) {
      console.error('REQUIRE FAILED:', e.message);
    }
    
    try {
      console.log('global type:', typeof global);
      if (typeof global !== 'undefined' && global.process) {
        console.log('global.process type:', typeof global.process);
        console.log('reallyExit:', typeof global.process.reallyExit);
        console.log('removeListener:', typeof global.process.removeListener);
        console.log('emit:', typeof global.process.emit);
        console.log('listeners:', typeof global.process.listeners);
        console.log('kill:', typeof global.process.kill);
        console.log('pid type:', typeof global.process.pid, 'val:', global.process.pid);
        console.log('on:', typeof global.process.on);
      }
    } catch(e) {
      console.error('GLOBAL CHECK FAILED:', e.message);
    }
  `);

  const r1 = await vm.exec("node /tmp/test.cjs");
  console.log("Exit code:", r1.exitCode);
  console.log("Stdout:", r1.stdout);
  console.log("Stderr:", r1.stderr);

  await vm.dispose();
}

main().catch(e => { console.error(e); process.exit(1); });
