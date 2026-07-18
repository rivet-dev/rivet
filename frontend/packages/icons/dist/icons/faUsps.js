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

// src/node_modules/@fortawesome/free-brands-svg-icons/faUsps.js
var require_faUsps = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faUsps.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "usps";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f7e1";
    var svgPathData = "M428.3 241.7c25.8-41.3 15.2-48.8-11.7-48.8l-27 0c-.1 0-1.5-1.4-10.9 8-11.2 5.6-37.9 6.3-37.9 8.7 0 4.5 70.3-3.1 88.1 0 9.5 1.5-1.5 20.4-4.4 32-.5 4.5 2.4 2.3 3.8 .1zM316.2 264.3c64-21.3 97.3-23.9 102-26.2 4.4-2.9-4.4-6.6-26.2-5.8-51.7 2.2-137.6 37.1-172.6 53.9l-30.7-93.3 196.6 0c-2.7-28.2-152.9-22.6-337.9-22.6L-5 415.8c196.4-97.3 258.9-130.3 321.2-151.5zM62.7 96c253.3 53.7 330 65.7 332.1 85.2 36.4 0 45.9 0 52.4 6.6 21.1 19.7-14.6 67.7-14.6 67.7-4.4 2.9-406.4 160.2-406.4 160.2L449.3 415.7 517 96 62.7 96z";
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
    exports.faUsps = exports.definition;
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

// src/_entries/faUsps.js
var import_faUsps = __toESM(require_faUsps());
var export_faUsps = import_faUsps.definition;
export {
  export_faUsps as faUsps
};
