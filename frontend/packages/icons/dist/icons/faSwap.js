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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSwap.js
var require_faSwap = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSwap.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "swap";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e609";
    var svgPathData = "M237 141.6c-5.3 11.2-16.6 18.4-29 18.4l-48 0 0 192c0 35.3 28.7 64 64 64s64-28.7 64-64l0-192c0-70.7 57.3-128 128-128s128 57.3 128 128l0 192 48 0c12.4 0 23.7 7.2 29 18.4s3.6 24.5-4.4 34.1l-80 96c-6.1 7.3-15.1 11.5-24.6 11.5s-18.5-4.2-24.6-11.5l-80-96c-7.9-9.5-9.7-22.8-4.4-34.1s16.6-18.4 29-18.4l48 0 0-192c0-35.3-28.7-64-64-64s-64 28.7-64 64l0 192c0 70.7-57.3 128-128 128s-128-57.3-128-128l0-192-48 0c-12.4 0-23.7-7.2-29-18.4s-3.6-24.5 4.4-34.1l80-96C109.5 4.2 118.5 0 128 0s18.5 4.2 24.6 11.5l80 96c7.9 9.5 9.7 22.8 4.4 34.1z";
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
    exports.faSwap = exports.definition;
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

// src/_entries/faSwap.js
var import_faSwap = __toESM(require_faSwap());
var export_faSwap = import_faSwap.definition;
export {
  export_faSwap as faSwap
};
