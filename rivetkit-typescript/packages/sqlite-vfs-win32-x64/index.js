const path = require("node:path");

const bindingPath = path.join(
	__dirname,
	"bin",
	"rivetkit_sqlite_vfs_native.node",
);

module.exports = require(bindingPath);
