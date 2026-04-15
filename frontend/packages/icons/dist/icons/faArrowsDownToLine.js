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

// src/node_modules/@fortawesome/free-solid-svg-icons/faArrowsDownToLine.js
var require_faArrowsDownToLine = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faArrowsDownToLine.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "arrows-down-to-line";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e4b8";
    var svgPathData = "M214.6 310.6l-64 64c-12.5 12.5-32.8 12.5-45.3 0l-64-64c-12.5-12.5-12.5-32.8 0-45.3s32.8-12.5 45.3 0L96 274.7 96 32c0-17.7 14.3-32 32-32s32 14.3 32 32l0 242.7 9.4-9.4c12.5-12.5 32.8-12.5 45.3 0s12.5 32.8 0 45.3zm256 0l-64 64c-12.5 12.5-32.8 12.5-45.3 0l-64-64c-12.5-12.5-12.5-32.8 0-45.3s32.8-12.5 45.3 0l9.4 9.4 0-242.7c0-17.7 14.3-32 32-32s32 14.3 32 32l0 242.7 9.4-9.4c12.5-12.5 32.8-12.5 45.3 0s12.5 32.8 0 45.3zM32 512c-17.7 0-32-14.3-32-32s14.3-32 32-32l448 0c17.7 0 32 14.3 32 32s-14.3 32-32 32L32 512z";
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
    exports.faArrowsDownToLine = exports.definition;
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

// src/_entries/faArrowsDownToLine.js
var import_faArrowsDownToLine = __toESM(require_faArrowsDownToLine());
var export_faArrowsDownToLine = import_faArrowsDownToLine.definition;
export {
  export_faArrowsDownToLine as faArrowsDownToLine
};
