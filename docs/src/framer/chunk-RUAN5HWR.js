// @ts-nocheck
/* eslint-disable */
var __defProp = Object.defineProperty;
var __export = (target, all,) => {
  for (var name in all) {
    __defProp(target, name, { get: all[name], enumerable: true, },);
  }
};

// ../../node_modules/@jspm/core/nodelibs/browser/process.js
var process_exports = {};
__export(process_exports, {
  _debugEnd: () => _debugEnd,
  _debugProcess: () => _debugProcess,
  _events: () => _events,
  _eventsCount: () => _eventsCount,
  _exiting: () => _exiting,
  _fatalExceptions: () => _fatalExceptions,
  _getActiveHandles: () => _getActiveHandles,
  _getActiveRequests: () => _getActiveRequests,
  _kill: () => _kill,
  _linkedBinding: () => _linkedBinding,
  _maxListeners: () => _maxListeners,
  _preload_modules: () => _preload_modules,
  _rawDebug: () => _rawDebug,
  _startProfilerIdleNotifier: () => _startProfilerIdleNotifier,
  _stopProfilerIdleNotifier: () => _stopProfilerIdleNotifier,
  _tickCallback: () => _tickCallback,
  abort: () => abort,
  addListener: () => addListener,
  allowedNodeEnvironmentFlags: () => allowedNodeEnvironmentFlags,
  arch: () => arch,
  argv: () => argv,
  argv0: () => argv0,
  assert: () => assert,
  binding: () => binding,
  browser: () => browser,
  chdir: () => chdir,
  config: () => config,
  cpuUsage: () => cpuUsage,
  cwd: () => cwd,
  debugPort: () => debugPort,
  default: () => process,
  dlopen: () => dlopen,
  domain: () => domain,
  emit: () => emit,
  emitWarning: () => emitWarning,
  env: () => env,
  execArgv: () => execArgv,
  execPath: () => execPath,
  exit: () => exit,
  features: () => features,
  hasUncaughtExceptionCaptureCallback: () => hasUncaughtExceptionCaptureCallback,
  hrtime: () => hrtime,
  kill: () => kill,
  listeners: () => listeners,
  memoryUsage: () => memoryUsage,
  moduleLoadList: () => moduleLoadList,
  nextTick: () => nextTick,
  off: () => off,
  on: () => on,
  once: () => once,
  openStdin: () => openStdin,
  pid: () => pid,
  platform: () => platform,
  ppid: () => ppid,
  prependListener: () => prependListener,
  prependOnceListener: () => prependOnceListener,
  reallyExit: () => reallyExit,
  release: () => release,
  removeAllListeners: () => removeAllListeners,
  removeListener: () => removeListener,
  resourceUsage: () => resourceUsage,
  setSourceMapsEnabled: () => setSourceMapsEnabled,
  setUncaughtExceptionCaptureCallback: () => setUncaughtExceptionCaptureCallback,
  stderr: () => stderr,
  stdin: () => stdin,
  stdout: () => stdout,
  title: () => title,
  umask: () => umask,
  uptime: () => uptime,
  version: () => version,
  versions: () => versions,
},);
function unimplemented(name,) {
  throw new Error('Node.js process ' + name + ' is not supported by JSPM core outside of Node.js',);
}
var queue = [];
var draining = false;
var currentQueue;
var queueIndex = -1;
function cleanUpNextTick() {
  if (!draining || !currentQueue) {
    return;
  }
  draining = false;
  if (currentQueue.length) {
    queue = currentQueue.concat(queue,);
  } else {
    queueIndex = -1;
  }
  if (queue.length) {
    drainQueue();
  }
}
function drainQueue() {
  if (draining) {
    return;
  }
  var timeout = setTimeout(cleanUpNextTick, 0,);
  draining = true;
  var len = queue.length;
  while (len) {
    currentQueue = queue;
    queue = [];
    while (++queueIndex < len) {
      if (currentQueue) {
        currentQueue[queueIndex].run();
      }
    }
    queueIndex = -1;
    len = queue.length;
  }
  currentQueue = null;
  draining = false;
  clearTimeout(timeout,);
}
function nextTick(fun,) {
  var args = new Array(arguments.length - 1,);
  if (arguments.length > 1) {
    for (var i = 1; i < arguments.length; i++) {
      args[i - 1] = arguments[i];
    }
  }
  queue.push(new Item(fun, args,),);
  if (queue.length === 1 && !draining) {
    setTimeout(drainQueue, 0,);
  }
}
function Item(fun, array,) {
  this.fun = fun;
  this.array = array;
}
Item.prototype.run = function () {
  this.fun.apply(null, this.array,);
};
var title = 'browser';
var arch = 'x64';
var platform = 'browser';
var env = {
  PATH: '/usr/bin',
  LANG: typeof navigator !== 'undefined' ? navigator.language + '.UTF-8' : void 0,
  PWD: '/',
  HOME: '/home',
  TMP: '/tmp',
};
var argv = ['/usr/bin/node',];
var execArgv = [];
var version = 'v16.8.0';
var versions = {};
var emitWarning = function (message, type,) {
  console.warn((type ? type + ': ' : '') + message,);
};
var binding = function (name,) {
  unimplemented('binding',);
};
var umask = function (mask,) {
  return 0;
};
var cwd = function () {
  return '/';
};
var chdir = function (dir,) {
};
var release = {
  name: 'node',
  sourceUrl: '',
  headersUrl: '',
  libUrl: '',
};
function noop() {
}
var browser = true;
var _rawDebug = noop;
var moduleLoadList = [];
function _linkedBinding(name,) {
  unimplemented('_linkedBinding',);
}
var domain = {};
var _exiting = false;
var config = {};
function dlopen(name,) {
  unimplemented('dlopen',);
}
function _getActiveRequests() {
  return [];
}
function _getActiveHandles() {
  return [];
}
var reallyExit = noop;
var _kill = noop;
var cpuUsage = function () {
  return {};
};
var resourceUsage = cpuUsage;
var memoryUsage = cpuUsage;
var kill = noop;
var exit = noop;
var openStdin = noop;
var allowedNodeEnvironmentFlags = {};
function assert(condition, message,) {
  if (!condition) throw new Error(message || 'assertion error',);
}
var features = {
  inspector: false,
  debug: false,
  uv: false,
  ipv6: false,
  tls_alpn: false,
  tls_sni: false,
  tls_ocsp: false,
  tls: false,
  cached_builtins: true,
};
var _fatalExceptions = noop;
var setUncaughtExceptionCaptureCallback = noop;
function hasUncaughtExceptionCaptureCallback() {
  return false;
}
var _tickCallback = noop;
var _debugProcess = noop;
var _debugEnd = noop;
var _startProfilerIdleNotifier = noop;
var _stopProfilerIdleNotifier = noop;
var stdout = void 0;
var stderr = void 0;
var stdin = void 0;
var abort = noop;
var pid = 2;
var ppid = 1;
var execPath = '/bin/usr/node';
var debugPort = 9229;
var argv0 = 'node';
var _preload_modules = [];
var setSourceMapsEnabled = noop;
var _performance = {
  now: typeof performance !== 'undefined' ? performance.now.bind(performance,) : void 0,
  timing: typeof performance !== 'undefined' ? performance.timing : void 0,
};
if (_performance.now === void 0) {
  nowOffset = Date.now();
  if (_performance.timing && _performance.timing.navigationStart) {
    nowOffset = _performance.timing.navigationStart;
  }
  _performance.now = () => Date.now() - nowOffset;
}
var nowOffset;
function uptime() {
  return _performance.now() / 1e3;
}
var nanoPerSec = 1e9;
function hrtime(previousTimestamp,) {
  var baseNow = Math.floor((Date.now() - _performance.now()) * 1e-3,);
  var clocktime = _performance.now() * 1e-3;
  var seconds = Math.floor(clocktime,) + baseNow;
  var nanoseconds = Math.floor(clocktime % 1 * 1e9,);
  if (previousTimestamp) {
    seconds = seconds - previousTimestamp[0];
    nanoseconds = nanoseconds - previousTimestamp[1];
    if (nanoseconds < 0) {
      seconds--;
      nanoseconds += nanoPerSec;
    }
  }
  return [seconds, nanoseconds,];
}
hrtime.bigint = function (time,) {
  var diff = hrtime(time,);
  if (typeof BigInt === 'undefined') {
    return diff[0] * nanoPerSec + diff[1];
  }
  return BigInt(diff[0] * nanoPerSec,) + BigInt(diff[1],);
};
var _maxListeners = 10;
var _events = {};
var _eventsCount = 0;
function on() {
  return process;
}
var addListener = on;
var once = on;
var off = on;
var removeListener = on;
var removeAllListeners = on;
var emit = noop;
var prependListener = on;
var prependOnceListener = on;
function listeners(name,) {
  return [];
}
var process = {
  version,
  versions,
  arch,
  platform,
  browser,
  release,
  _rawDebug,
  moduleLoadList,
  binding,
  _linkedBinding,
  _events,
  _eventsCount,
  _maxListeners,
  on,
  addListener,
  once,
  off,
  removeListener,
  removeAllListeners,
  emit,
  prependListener,
  prependOnceListener,
  listeners,
  domain,
  _exiting,
  config,
  dlopen,
  uptime,
  _getActiveRequests,
  _getActiveHandles,
  reallyExit,
  _kill,
  cpuUsage,
  resourceUsage,
  memoryUsage,
  kill,
  exit,
  openStdin,
  allowedNodeEnvironmentFlags,
  assert,
  features,
  _fatalExceptions,
  setUncaughtExceptionCaptureCallback,
  hasUncaughtExceptionCaptureCallback,
  emitWarning,
  nextTick,
  _tickCallback,
  _debugProcess,
  _debugEnd,
  _startProfilerIdleNotifier,
  _stopProfilerIdleNotifier,
  stdout,
  stdin,
  stderr,
  abort,
  umask,
  chdir,
  cwd,
  env,
  title,
  argv,
  execArgv,
  pid,
  ppid,
  execPath,
  debugPort,
  hrtime,
  argv0,
  _preload_modules,
  setSourceMapsEnabled,
};

export { process_exports, };
