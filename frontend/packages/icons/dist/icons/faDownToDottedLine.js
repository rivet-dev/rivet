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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faDownToDottedLine.js
var require_faDownToDottedLine = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faDownToDottedLine.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "down-to-dotted-line";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e408";
    var svgPathData = "M0 448a32 32 0 1 0 64 0A32 32 0 1 0 0 448zm128 32a32 32 0 1 0 0-64 32 32 0 1 0 0 64zm64-32a32 32 0 1 0 64 0 32 32 0 1 0 -64 0zm128 32a32 32 0 1 0 0-64 32 32 0 1 0 0 64zm96 0a32 32 0 1 0 0-64 32 32 0 1 0 0 64zM73.9 249.9L196.7 372.7c7.2 7.2 17.1 11.3 27.3 11.3s20.1-4.1 27.3-11.3L374.1 249.9c6.4-6.4 9.9-15 9.9-24c0-18.7-15.2-33.9-33.9-33.9L288 192l0-128c0-17.7-14.3-32-32-32l-64 0c-17.7 0-32 14.3-32 32l0 128-62.1 0C79.2 192 64 207.2 64 225.9c0 9 3.6 17.6 9.9 24z";
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
    exports.faDownToDottedLine = exports.definition;
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

// src/_entries/faDownToDottedLine.js
var import_faDownToDottedLine = __toESM(require_faDownToDottedLine());
var export_faDownToDottedLine = import_faDownToDottedLine.definition;
export {
  export_faDownToDottedLine as faDownToDottedLine
};
