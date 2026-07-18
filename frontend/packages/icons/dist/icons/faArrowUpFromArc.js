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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faArrowUpFromArc.js
var require_faArrowUpFromArc = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faArrowUpFromArc.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "arrow-up-from-arc";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e4b4";
    var svgPathData = "M256 448C150 448 64 362 64 256c0-17.7-14.3-32-32-32s-32 14.3-32 32C0 397.4 114.6 512 256 512s256-114.6 256-256c0-17.7-14.3-32-32-32s-32 14.3-32 32c0 106-86 192-192 192zM121.4 121.4c-12.5 12.5-12.5 32.8 0 45.3s32.8 12.5 45.3 0L224 109.3 224 320c0 17.7 14.3 32 32 32s32-14.3 32-32l0-210.7 57.4 57.4c12.5 12.5 32.8 12.5 45.3 0s12.5-32.8 0-45.3l-112-112c-12.5-12.5-32.8-12.5-45.3 0l-112 112z";
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
    exports.faArrowUpFromArc = exports.definition;
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

// src/_entries/faArrowUpFromArc.js
var import_faArrowUpFromArc = __toESM(require_faArrowUpFromArc());
var export_faArrowUpFromArc = import_faArrowUpFromArc.definition;
export {
  export_faArrowUpFromArc as faArrowUpFromArc
};
