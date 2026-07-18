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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFileMp4.js
var require_faFileMp4 = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFileMp4.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "file-mp4";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e649";
    var svgPathData = "M0 64C0 28.7 28.7 0 64 0L224 0l0 128c0 17.7 14.3 32 32 32l128 0 0 144-240 0c-35.3 0-64 28.7-64 64l0 144-16 0c-35.3 0-64-28.7-64-64L0 64zm384 64l-128 0L256 0 384 128zM157.7 359.8L192 416.9l34.3-57.1c3.7-6.2 11.1-9.1 18-7.2s11.7 8.2 11.7 15.4l0 128c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-70.2-18.3 30.5c-2.9 4.8-8.1 7.8-13.7 7.8s-10.8-3-13.7-7.8L160 425.8l0 70.2c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-128c0-7.2 4.8-13.5 11.7-15.4s14.3 1 18 7.2zM304 352l32 0c30.9 0 56 25.1 56 56s-25.1 56-56 56l-16 0 0 32c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-48 0-80c0-8.8 7.2-16 16-16zm32 80c13.3 0 24-10.7 24-24s-10.7-24-24-24l-16 0 0 48 16 0zm84.7 11.3c-3-3-4.7-7.1-4.7-11.3l0-64c0-8.8 7.2-16 16-16s16 7.2 16 16l0 48 32 0 0-48c0-8.8 7.2-16 16-16s16 7.2 16 16l0 64 0 64c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-48-48 0c-4.2 0-8.3-1.7-11.3-4.7z";
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
    exports.faFileMp4 = exports.definition;
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

// src/_entries/faFileMp4.js
var import_faFileMp4 = __toESM(require_faFileMp4());
var export_faFileMp4 = import_faFileMp4.definition;
export {
  export_faFileMp4 as faFileMp4
};
