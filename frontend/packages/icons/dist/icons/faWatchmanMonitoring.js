var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));

// src/node_modules/@fortawesome/free-brands-svg-icons/faWatchmanMonitoring.js
var require_faWatchmanMonitoring = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faWatchmanMonitoring.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "watchman-monitoring";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e087";
    var svgPathData = "M256 16a240 240 0 1 0 0 480 240 240 0 1 0 0-480zM121.7 429.1c-51.6-40.2-84.9-102.8-84.9-173.1 0-21.7 3.2-43.3 9.6-64.1l102.9-17.9-.1 11-13.9 2s-.1 12.5-.1 19.5c0 2 .4 4 1.2 5.8s2.1 3.4 3.7 4.6l9.5 7.4-27.7 204.9zM227.4 145.9l8.5-7.6s6.9-5.4-.1-9.3c-7.2-4-39.5-34.5-39.5-34.5-5.3-5.5-8.3-7.3-15.5 0 0 0-32.3 30.5-39.5 34.5-7.1 4-.1 9.3-.1 9.3l8.5 7.6 0 4.4-73.5-19.2c39.6-56.9 105.5-94.3 180-94.3 31.3 0 62.2 6.7 90.6 19.6s53.8 31.8 74.3 55.4l-193.5 37.7 0-3.6zm34.1 329.3l-33.9-250.9 9.5-7.4c1.6-1.2 2.8-2.8 3.7-4.6s1.3-3.8 1.2-5.8c0-7-.1-19.5-.1-19.5l-13.9-2-.1-10.5 241.7 31.4c3.9 16.4 5.8 33.3 5.8 50.1 0 119.1-95.4 216.2-213.8 219.1z";
    exports.definition = {
      prefix,
      iconName,
      icon: [
        width,
        height,
        aliases,
        unicode,
        svgPathData
      ]
    };
    exports.faWatchmanMonitoring = exports.definition;
    exports.prefix = prefix;
    exports.iconName = iconName;
    exports.width = width;
    exports.height = height;
    exports.ligatures = aliases;
    exports.unicode = unicode;
    exports.svgPathData = svgPathData;
    exports.aliases = aliases;
  }
});

// src/_entries/faWatchmanMonitoring.js
var import_faWatchmanMonitoring = __toESM(require_faWatchmanMonitoring());
var export_faWatchmanMonitoring = import_faWatchmanMonitoring.definition;
export {
  export_faWatchmanMonitoring as faWatchmanMonitoring
};
