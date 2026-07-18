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

// src/node_modules/@fortawesome/free-solid-svg-icons/faHandsBound.js
var require_faHandsBound = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faHandsBound.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "hands-bound";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e4f9";
    var svgPathData = "M64 32C64 14.3 49.7 0 32 0S0 14.3 0 32L0 213.9c0 14.2 5.1 27.9 14.3 38.7L99.6 352 96 352c-13.3 0-24 10.7-24 24s10.7 24 24 24l384 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-3.6 0 85.3-99.5c9.2-10.8 14.3-24.5 14.3-38.7L576 32c0-17.7-14.3-32-32-32s-32 14.3-32 32l0 112.8-69.3 92.4c-5.7 7.6-16.1 9.6-24.2 4.8-9.7-5.7-12.1-18.7-5.1-27.5L441 180c10.8-13.5 8.9-33.3-4.4-44.5s-33-9.8-44.5 3.2l-46.7 52.5C329 209.7 320 233.4 320 258.1l0 93.9-64 0 0-93.9c0-24.6-9-48.4-25.4-66.8l-46.7-52.5c-11.5-13-31.3-14.4-44.5-3.2S124.2 166.4 135 180l27.6 34.5c7 8.8 4.7 21.8-5.1 27.5-8.1 4.8-18.6 2.7-24.2-4.8L64 144.8 64 32zm64 448l0 32 128 0 0-32 64 0 0 32 128 0 0-32 32 0c13.3 0 24-10.7 24-24s-10.7-24-24-24L96 432c-13.3 0-24 10.7-24 24s10.7 24 24 24l32 0z";
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
    exports.faHandsBound = exports.definition;
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

// src/_entries/faHandsBound.js
var import_faHandsBound = __toESM(require_faHandsBound());
var export_faHandsBound = import_faHandsBound.definition;
export {
  export_faHandsBound as faHandsBound
};
