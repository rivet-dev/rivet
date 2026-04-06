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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCabin.js
var require_faCabin = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCabin.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "cabin";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e46d";
    var svgPathData = "M237.4 6C248.5-2 263.5-2 274.6 6L384 84.1 384 64c0-17.7 14.3-32 32-32l32 0c17.7 0 32 14.3 32 32l0 88.7L498.6 166c11.3 8.1 16.1 22.6 11.9 35.8S493.9 224 480 224l-32 0-32 0L32 224c-13.9 0-26.2-9-30.5-22.2S2.1 174 13.4 166L237.4 6zM32 280c0-13.3 10.7-24 24-24l112 0c13.3 0 24 10.7 24 24l0 16c0 13.3-10.7 24-24 24L56 320c-13.3 0-24-10.7-24-24l0-16zm24 72l112 0c13.3 0 24 10.7 24 24l0 16c0 13.3-10.7 24-24 24L56 416c-13.3 0-24-10.7-24-24l0-16c0-13.3 10.7-24 24-24zm0 96l112 0c13.3 0 24 10.7 24 24l0 16c0 13.3-10.7 24-24 24L56 512c-13.3 0-24-10.7-24-24l0-16c0-13.3 10.7-24 24-24zM320 280c0-13.3 10.7-24 24-24l112 0c13.3 0 24 10.7 24 24l0 16c0 13.3-10.7 24-24 24l-112 0c-13.3 0-24-10.7-24-24l0-16zm24 168l112 0c13.3 0 24 10.7 24 24l0 16c0 13.3-10.7 24-24 24l-112 0c-13.3 0-24-10.7-24-24l0-16c0-13.3 10.7-24 24-24zm-24-72c0-13.3 10.7-24 24-24l112 0c13.3 0 24 10.7 24 24l0 16c0 13.3-10.7 24-24 24l-112 0c-13.3 0-24-10.7-24-24l0-16zM256 160a32 32 0 1 0 0-64 32 32 0 1 0 0 64z";
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
    exports.faCabin = exports.definition;
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

// src/_entries/faCabin.js
var import_faCabin = __toESM(require_faCabin());
var export_faCabin = import_faCabin.definition;
export {
  export_faCabin as faCabin
};
