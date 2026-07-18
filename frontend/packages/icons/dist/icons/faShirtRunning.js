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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faShirtRunning.js
var require_faShirtRunning = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faShirtRunning.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "shirt-running";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "e3c8";
    var svgPathData = "M80 0c17.7 0 32 14.3 32 32l0 48c0 44.2 35.8 80 80 80s80-35.8 80-80l0-48c0-17.7 14.3-32 32-32s32 14.3 32 32l0 32c0 33.3 2.6 61.5 6.5 85.2L0 393.8 0 282.5c0-17 6.7-33.5 14.7-48.4C28.8 207.6 48 155 48 64l0-32C48 14.3 62.3 0 80 0zM3.8 469.8L360.4 215c3 7.5 6 13.8 8.9 19c8 15 14.7 31.5 14.7 48.4L384 448c0 35.3-28.7 64-64 64L64 512c-27.7 0-51.3-17.6-60.2-42.2z";
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
    exports.faShirtRunning = exports.definition;
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

// src/_entries/faShirtRunning.js
var import_faShirtRunning = __toESM(require_faShirtRunning());
var export_faShirtRunning = import_faShirtRunning.definition;
export {
  export_faShirtRunning as faShirtRunning
};
