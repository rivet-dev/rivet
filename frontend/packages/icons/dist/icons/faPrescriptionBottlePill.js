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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPrescriptionBottlePill.js
var require_faPrescriptionBottlePill = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPrescriptionBottlePill.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "prescription-bottle-pill";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e5c0";
    var svgPathData = "M0 32C0 14.3 14.3 0 32 0L352 0c17.7 0 32 14.3 32 32l0 32c0 17.7-14.3 32-32 32L32 96C14.3 96 0 81.7 0 64L0 32zm32 96l320 0 0 83.2c-57 29.1-96 88.4-96 156.8c0 54.5 24.8 103.3 63.8 135.6c-9.4 5.4-20.2 8.4-31.8 8.4L96 512c-35.3 0-64-28.7-64-64l0-32 112 0c8.8 0 16-7.2 16-16s-7.2-16-16-16L32 384l0-64 112 0c8.8 0 16-7.2 16-16s-7.2-16-16-16L32 288l0-64 112 0c8.8 0 16-7.2 16-16s-7.2-16-16-16L32 192l0-64zM304.8 300.5c6.7-12.6 23.6-13.5 33.7-3.4L502.9 461.5c10.1 10.1 9.2 27-3.4 33.7C479.4 505.9 456.4 512 432 512c-79.5 0-144-64.5-144-144c0-24.4 6.1-47.4 16.8-67.5zm254.5 135c-6.7 12.6-23.6 13.5-33.7 3.4L361.1 274.5c-10.1-10.1-9.2-27 3.4-33.7C384.6 230.1 407.6 224 432 224c79.5 0 144 64.5 144 144c0 24.4-6.1 47.4-16.8 67.5z";
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
    exports.faPrescriptionBottlePill = exports.definition;
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

// src/_entries/faPrescriptionBottlePill.js
var import_faPrescriptionBottlePill = __toESM(require_faPrescriptionBottlePill());
var export_faPrescriptionBottlePill = import_faPrescriptionBottlePill.definition;
export {
  export_faPrescriptionBottlePill as faPrescriptionBottlePill
};
