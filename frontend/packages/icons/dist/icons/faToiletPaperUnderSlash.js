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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faToiletPaperUnderSlash.js
var require_faToiletPaperUnderSlash = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faToiletPaperUnderSlash.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "toilet-paper-under-slash";
    var width = 640;
    var height = 512;
    var aliases = ["toilet-paper-reverse-slash"];
    var unicode = "e2a1";
    var svgPathData = "M38.8 5.1C28.4-3.1 13.3-1.2 5.1 9.2S-1.2 34.7 9.2 42.9l592 464c10.4 8.2 25.5 6.3 33.7-4.1s6.3-25.5-4.1-33.7l-54.8-43 0-83.1c-9.9 19.5-23.2 38.2-39.9 51.8L510 374.4c22.6-14.8 41.4-46 53.1-86.4c8.2-28.2 12.8-61 12.8-96C576 86 533 0 480 0c-11.2 0-22 3.8-32 10.9c-37.3 26.4-64 97.5-64 181.1c0 33.8 4.4 65.5 12 93.1l-39.3-30.8c-3.1-19.9-4.7-40.8-4.7-62.3c0-56.9 11.5-110 31.5-150.1c7.5-15 17.1-29.7 28.7-41.9L160 0c-24.8 0-47.4 18.8-64.4 49.6L38.8 5.1zM528 512L406.2 416 224 416l0 48c0 26.5 21.5 48 48 48l256 0 .3 0-.3 0zM64 192c0 106 43 192 96 192l205.5 0L66.5 148.4C64.9 162.4 64 177 64 192zm416 64c-17.7 0-32-28.7-32-64s14.3-64 32-64s32 28.7 32 64s-14.3 64-32 64z";
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
    exports.faToiletPaperUnderSlash = exports.definition;
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

// src/_entries/faToiletPaperUnderSlash.js
var import_faToiletPaperUnderSlash = __toESM(require_faToiletPaperUnderSlash());
var export_faToiletPaperUnderSlash = import_faToiletPaperUnderSlash.definition;
export {
  export_faToiletPaperUnderSlash as faToiletPaperUnderSlash
};
