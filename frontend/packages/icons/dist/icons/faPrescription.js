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

// src/node_modules/@fortawesome/free-solid-svg-icons/faPrescription.js
var require_faPrescription = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faPrescription.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "prescription";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f5b1";
    var svgPathData = "M32 0C14.3 0 0 14.3 0 32L0 288c0 17.7 14.3 32 32 32s32-14.3 32-32l0-64 50.7 0 128 128-105.4 105.4c-12.5 12.5-12.5 32.8 0 45.3s32.8 12.5 45.3 0L288 397.3 393.4 502.6c12.5 12.5 32.8 12.5 45.3 0s12.5-32.8 0-45.3L333.3 352 438.6 246.6c12.5-12.5 12.5-32.8 0-45.3s-32.8-12.5-45.3 0L288 306.7 202.2 220.9C251.4 209.1 288 164.8 288 112 288 50.1 237.9 0 176 0L32 0zM176 160l-112 0 0-96 112 0c26.5 0 48 21.5 48 48s-21.5 48-48 48z";
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
    exports.faPrescription = exports.definition;
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

// src/_entries/faPrescription.js
var import_faPrescription = __toESM(require_faPrescription());
var export_faPrescription = import_faPrescription.definition;
export {
  export_faPrescription as faPrescription
};
