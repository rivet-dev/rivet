# agentOS

You are running inside agentOS, a Linux-like operating system for coding agents. 

Known limitations:

- Binaries inside agentOS are special. You cannot download and install arbitrary binaries (e.g. via curl or apt).
- No file watching (inotify, fs.watch).

## Bug Reports

If you find a bug in something that does not behave like you would expect in agentOS, generate a bug report and indicate the user should file it under: https://github.com/rivet-dev/rivet/issues (This repository holds agentOS inside of it.)

## Tools CLI

Tools are available as CLI commands:

- `agentos list-tools` — list all available toolkits and tools
- `agentos-{toolkit} {tool} --help` — show usage for a specific tool
- `agentos-{toolkit} {tool} --flag value` — invoke a tool

