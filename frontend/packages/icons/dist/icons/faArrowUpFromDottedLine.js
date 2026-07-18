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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faArrowUpFromDottedLine.js
var require_faArrowUpFromDottedLine = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faArrowUpFromDottedLine.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "arrow-up-from-dotted-line";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e09b";
    var svgPathData = "M246.6 41.4c-12.5-12.5-32.8-12.5-45.3 0l-128 128c-12.5 12.5-12.5 32.8 0 45.3s32.8 12.5 45.3 0L192 141.3l0 66.7 0 112c0 17.7 14.3 32 32 32s32-14.3 32-32l0-112 0-66.7 73.4 73.4c12.5 12.5 32.8 12.5 45.3 0s12.5-32.8 0-45.3l-128-128zM32 416a32 32 0 1 0 0 64 32 32 0 1 0 0-64zm96 0a32 32 0 1 0 0 64 32 32 0 1 0 0-64zm128 32a32 32 0 1 0 -64 0 32 32 0 1 0 64 0zm64-32a32 32 0 1 0 0 64 32 32 0 1 0 0-64zm64 32a32 32 0 1 0 64 0 32 32 0 1 0 -64 0z";
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
    exports.faArrowUpFromDottedLine = exports.definition;
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

// src/_entries/faArrowUpFromDottedLine.js
var import_faArrowUpFromDottedLine = __toESM(require_faArrowUpFromDottedLine());
var export_faArrowUpFromDottedLine = import_faArrowUpFromDottedLine.definition;
export {
  export_faArrowUpFromDottedLine as faArrowUpFromDottedLine
};
