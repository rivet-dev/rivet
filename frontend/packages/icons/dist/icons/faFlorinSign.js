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

// src/node_modules/@fortawesome/free-solid-svg-icons/faFlorinSign.js
var require_faFlorinSign = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faFlorinSign.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "florin-sign";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "e184";
    var svgPathData = "M314.7 32c-38.8 0-73.7 23.3-88.6 59.1L170.7 224 64 224c-17.7 0-32 14.3-32 32s14.3 32 32 32l80 0-45.1 108.3c-5 11.9-16.6 19.7-29.5 19.7L32 416c-17.7 0-32 14.3-32 32s14.3 32 32 32l37.3 0c38.8 0 73.7-23.3 88.6-59.1L213.3 288 320 288c17.7 0 32-14.3 32-32s-14.3-32-32-32l-80 0 45.1-108.3c5-11.9 16.6-19.7 29.5-19.7L352 96c17.7 0 32-14.3 32-32s-14.3-32-32-32l-37.3 0z";
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
    exports.faFlorinSign = exports.definition;
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

// src/_entries/faFlorinSign.js
var import_faFlorinSign = __toESM(require_faFlorinSign());
var export_faFlorinSign = import_faFlorinSign.definition;
export {
  export_faFlorinSign as faFlorinSign
};
