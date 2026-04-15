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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faHighlighterLine.js
var require_faHighlighterLine = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faHighlighterLine.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "highlighter-line";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e1af";
    var svgPathData = "M229 229l215-158.4 29.3 29.3L315 315 229 229zM128 320s0 0 0 0l-25.4 25.4c-12.5 12.5-12.5 32.8 0 45.3l50.7 50.7c12.5 12.5 32.8 12.5 45.3 0L224 416l71.7 0c15.3 0 29.6-7.2 38.6-19.5L535.6 123.4c5.5-7.4 8.4-16.4 8.4-25.6c0-11.4-4.5-22.4-12.6-30.5L476.6 12.6C468.5 4.5 457.6 0 446.2 0C437 0 428 2.9 420.6 8.4L147.5 209.6c-12.3 9-19.5 23.4-19.5 38.6l0 71.7zM70.1 403.3l-63 63c-4.5 4.5-7 10.6-7 17L0 488c0 13.3 10.7 24 24 24l68.7 0c6.4 0 12.5-2.5 17-7l31-31L70.1 403.3zM224 464c-13.3 0-24 10.7-24 24s10.7 24 24 24l328 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-328 0z";
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
    exports.faHighlighterLine = exports.definition;
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

// src/_entries/faHighlighterLine.js
var import_faHighlighterLine = __toESM(require_faHighlighterLine());
var export_faHighlighterLine = import_faHighlighterLine.definition;
export {
  export_faHighlighterLine as faHighlighterLine
};
