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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faArrowDownToBracket.js
var require_faArrowDownToBracket = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faArrowDownToBracket.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "arrow-down-to-bracket";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e094";
    var svgPathData = "M374.6 214.6l-128 128c-12.5 12.5-32.8 12.5-45.3 0l-128-128c-12.5-12.5-12.5-32.8 0-45.3s32.8-12.5 45.3 0L192 242.7 192 32c0-17.7 14.3-32 32-32s32 14.3 32 32l0 210.7 73.4-73.4c12.5-12.5 32.8-12.5 45.3 0s12.5 32.8 0 45.3zM64 352l0 64c0 17.7 14.3 32 32 32l256 0c17.7 0 32-14.3 32-32l0-64c0-17.7 14.3-32 32-32s32 14.3 32 32l0 64c0 53-43 96-96 96L96 512c-53 0-96-43-96-96l0-64c0-17.7 14.3-32 32-32s32 14.3 32 32z";
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
    exports.faArrowDownToBracket = exports.definition;
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

// src/_entries/faArrowDownToBracket.js
var import_faArrowDownToBracket = __toESM(require_faArrowDownToBracket());
var export_faArrowDownToBracket = import_faArrowDownToBracket.definition;
export {
  export_faArrowDownToBracket as faArrowDownToBracket
};
