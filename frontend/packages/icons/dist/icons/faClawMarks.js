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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faClawMarks.js
var require_faClawMarks = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faClawMarks.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "claw-marks";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f6c2";
    var svgPathData = "M41.4 9.4C53.9-3.1 74.1-3.1 86.6 9.4l96 96 64 64 160 160 64 64 64 64c12.5 12.5 12.5 32.8 0 45.3s-32.8 12.5-45.3 0L434.7 448 384 448c-17.7 0-32-14.3-32-32l0-50.7L210.7 224 160 224c-17.7 0-32-14.3-32-32l0-50.7L41.4 54.6c-12.5-12.5-12.5-32.8 0-45.3zm0 224c12.5-12.5 32.8-12.5 45.3 0l64 64 96 96 64 64c12.5 12.5 12.5 32.8 0 45.3s-32.8 12.5-45.3 0L210.7 448 128 448c-17.7 0-32-14.3-32-32l0-82.7L41.4 278.6c-12.5-12.5-12.5-32.8 0-45.3zM310.6 9.4l32 32 64 64 64 64 64 64 32 32c12.5 12.5 12.5 32.8 0 45.3s-32.8 12.5-45.3 0L498.7 288 448 288c-17.7 0-32-14.3-32-32l0-50.7L370.7 160 320 160c-17.7 0-32-14.3-32-32l0-50.7L265.4 54.6c-12.5-12.5-12.5-32.8 0-45.3s32.8-12.5 45.3 0z";
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
    exports.faClawMarks = exports.definition;
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

// src/_entries/faClawMarks.js
var import_faClawMarks = __toESM(require_faClawMarks());
var export_faClawMarks = import_faClawMarks.definition;
export {
  export_faClawMarks as faClawMarks
};
