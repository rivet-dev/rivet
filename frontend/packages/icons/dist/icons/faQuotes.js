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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faQuotes.js
var require_faQuotes = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faQuotes.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "quotes";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e234";
    var svgPathData = "M88 0C39.4 0 0 39.4 0 88l0 56 0 24 0 24c0 26.5 21.5 48 48 48l48 0c26.5 0 48-21.5 48-48l0-48c0-26.5-21.5-48-48-48L64 96l0-8c0-13.3 10.7-24 24-24l8 0c17.7 0 32-14.3 32-32s-14.3-32-32-32L88 0zM264 0c-48.6 0-88 39.4-88 88l0 56 0 24 0 24c0 26.5 21.5 48 48 48l48 0c26.5 0 48-21.5 48-48l0-48c0-26.5-21.5-48-48-48l-32 0 0-8c0-13.3 10.7-24 24-24l8 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-8 0zM576 424l0-56 0-24 0-24c0-26.5-21.5-48-48-48l-48 0c-26.5 0-48 21.5-48 48l0 48c0 26.5 21.5 48 48 48l32 0 0 8c0 13.3-10.7 24-24 24l-8 0c-17.7 0-32 14.3-32 32s14.3 32 32 32l8 0c48.6 0 88-39.4 88-88zM312 512c48.6 0 88-39.4 88-88l0-56 0-24 0-24c0-26.5-21.5-48-48-48l-48 0c-26.5 0-48 21.5-48 48l0 48c0 26.5 21.5 48 48 48l32 0 0 8c0 13.3-10.7 24-24 24l-8 0c-17.7 0-32 14.3-32 32s14.3 32 32 32l8 0z";
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
    exports.faQuotes = exports.definition;
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

// src/_entries/faQuotes.js
var import_faQuotes = __toESM(require_faQuotes());
var export_faQuotes = import_faQuotes.definition;
export {
  export_faQuotes as faQuotes
};
