const EXIT_CODE_MESSAGE: { [code: number]: string } = {
	"0": "Graceful exit",
	"1": "Generic error",
	"2": "Missing keyword or command/Permission problem",
	"3": "Node.js: Internal JavaScript parse error",
	"4": "Node.js: Internal JavaScript evaluation failure",
	"5": "Node.js: Fatal error",
	"6": "Node.js: Non-function internal exception handler",
	"7": "Node.js: Internal exception handler run-time failure",
	"9": "Node.js: Invalid argument",
	"10": "Node.js: Internal JavaScript run-time failure",
	"12": "Node.js: Invalid debug argument",
	"13": "Node.js: Unfinished top-level await",
	"124": "Command time out",
	"126": "Permission problem, command probably not executable",
	"127": "Command not found",
	"130": "Identity terminated",
	"137": "Out of memory",
};

// https://en.wikipedia.org/wiki/Signal_(IPC)#Default_action
const EXIT_CODE_SIG: { [signal: number]: string } = {
	"1": "SIGHUP",
	"2": "SIGINT",
	"3": "SIGQUIT",
	"4": "SIGILL",
	"5": "SIGTRAP",
	"6": "SIGABRT",
	"8": "SIGFPE",
	"9": "SIGKILL",
	"11": "SIGSEGV",
	"13": "SIGPIPE",
	"14": "SIGALRM",
	"15": "SIGTERM",
};

// https://tldp.org/LDP/abs/html/exitcodes.html
// https://www.tutorialspoint.com/unix/unix-signals-traps.htm
// https://nodejs.org/api/process.html#process_exit_codes
// https://unix.stackexchange.com/a/254747
// https://stackoverflow.com/questions/42972908/container-killed-by-the-applicationmaster-exit-code-is-143
export function formatExitCodeMessage(exitCode: number) {
	let msg = EXIT_CODE_MESSAGE[exitCode];
	if (EXIT_CODE_SIG[exitCode - 128]) {
		const signal = EXIT_CODE_SIG[exitCode - 128];
		if (msg != null) {
			msg = `${signal}: ${msg}`;
		} else {
			msg = signal;
		}
	}

	return msg;
}
