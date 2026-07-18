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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faMistletoe.js
var require_faMistletoe = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faMistletoe.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "mistletoe";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f7b4";
    var svgPathData = "M312 24c0-13.3-10.7-24-24-24s-24 10.7-24 24l0 94.1-98 98L91.3 230.4C38.3 240.6 0 287 0 341c0 41.4 33.6 75 75 75c50.5 0 94.3-33.5 108.1-81.2c7.2 10.4 19.3 17.2 32.9 17.2c22.1 0 40-17.9 40-40s-17.9-40-40-40c-7.9 0-15.3 2.3-21.5 6.3l5.4-28.2L264 185.9l0 72.4c14.7 13.2 24 32.3 24 53.7c0 39.8-32.2 72-72 72c-7.2 0-14.2-1.1-20.8-3c-2.1 8.4-3.2 17.1-3.2 25.9l0 9.1c0 53 43 96 96 96s96-43 96-96l0-9.1c0-25.1-9-49.4-25.3-68.4L312 284l0-98.1L376.1 250l14.4 74.7C400.6 377.7 447 416 501 416c41.4 0 75-33.6 75-75c0-54-38.3-100.3-91.3-110.5L410 216.1l-98-98L312 24zm64 88a40 40 0 1 0 0-80 40 40 0 1 0 0 80z";
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
    exports.faMistletoe = exports.definition;
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

// src/_entries/faMistletoe.js
var import_faMistletoe = __toESM(require_faMistletoe());
var export_faMistletoe = import_faMistletoe.definition;
export {
  export_faMistletoe as faMistletoe
};
