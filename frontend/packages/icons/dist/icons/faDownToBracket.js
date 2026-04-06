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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faDownToBracket.js
var require_faDownToBracket = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faDownToBracket.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "down-to-bracket";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e4e7";
    var svgPathData = "M378.1 198.6L249.5 341.4c-6.1 6.7-14.7 10.6-23.8 10.6l-3.5 0c-9.1 0-17.7-3.8-23.8-10.6L69.9 198.6c-3.8-4.2-5.9-9.8-5.9-15.5C64 170.4 74.4 160 87.1 160l72.9 0 0-128c0-17.7 14.3-32 32-32l64 0c17.7 0 32 14.3 32 32l0 128 72.9 0c12.8 0 23.1 10.4 23.1 23.1c0 5.7-2.1 11.2-5.9 15.5zM64 352l0 64c0 17.7 14.3 32 32 32l256 0c17.7 0 32-14.3 32-32l0-64c0-17.7 14.3-32 32-32s32 14.3 32 32l0 64c0 53-43 96-96 96L96 512c-53 0-96-43-96-96l0-64c0-17.7 14.3-32 32-32s32 14.3 32 32z";
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
    exports.faDownToBracket = exports.definition;
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

// src/_entries/faDownToBracket.js
var import_faDownToBracket = __toESM(require_faDownToBracket());
var export_faDownToBracket = import_faDownToBracket.definition;
export {
  export_faDownToBracket as faDownToBracket
};
