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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPiano.js
var require_faPiano = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPiano.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "piano";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f8d4";
    var svgPathData = "M0 192L0 448c0 35.3 28.7 64 64 64l384 0c35.3 0 64-28.7 64-64l0-120.4c0-24.2-13.7-46.4-35.4-57.2l-58.4-29.2c-21-10.5-34.2-31.9-34.2-55.4C384 83.2 300.8 0 198.3 0L192 0C86 0 0 86 0 192zM416 352l32 0 0 96L64 448l0-96 32 0 0 48c0 8.8 7.2 16 16 16s16-7.2 16-16l0-48 32 0 0 48c0 8.8 7.2 16 16 16s16-7.2 16-16l0-48 32 0 0 48c0 8.8 7.2 16 16 16s16-7.2 16-16l0-48 64 0 0 48c0 8.8 7.2 16 16 16s16-7.2 16-16l0-48 32 0 0 48c0 8.8 7.2 16 16 16s16-7.2 16-16l0-48z";
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
    exports.faPiano = exports.definition;
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

// src/_entries/faPiano.js
var import_faPiano = __toESM(require_faPiano());
var export_faPiano = import_faPiano.definition;
export {
  export_faPiano as faPiano
};
