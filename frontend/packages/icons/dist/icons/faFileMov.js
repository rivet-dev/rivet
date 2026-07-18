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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFileMov.js
var require_faFileMov = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFileMov.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "file-mov";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e647";
    var svgPathData = "M0 64C0 28.7 28.7 0 64 0L224 0l0 128c0 17.7 14.3 32 32 32l128 0 0 144-240 0c-35.3 0-64 28.7-64 64l0 144-16 0c-35.3 0-64-28.7-64-64L0 64zm384 64l-128 0L256 0 384 128zM157.7 359.8L192 416.9l34.3-57.1c3.7-6.2 11.1-9.1 18-7.2s11.7 8.2 11.7 15.4l0 128c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-70.2-18.3 30.5c-2.9 4.8-8.1 7.8-13.7 7.8s-10.8-3-13.7-7.8L160 425.8l0 70.2c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-128c0-7.2 4.8-13.5 11.7-15.4s14.3 1 18 7.2zM448 368l0 31.6c0 23 5.5 45.6 16 66c10.5-20.3 16-42.9 16-66l0-31.6c0-8.8 7.2-16 16-16s16 7.2 16 16l0 31.6c0 34.7-10.3 68.7-29.6 97.6l-5.1 7.7c-3 4.5-8 7.1-13.3 7.1s-10.3-2.7-13.3-7.1l-5.1-7.7c-19.3-28.9-29.6-62.9-29.6-97.6l0-31.6c0-8.8 7.2-16 16-16s16 7.2 16 16zM288 392c0-22.1 17.9-40 40-40l16 0c22.1 0 40 17.9 40 40l0 80c0 22.1-17.9 40-40 40l-16 0c-22.1 0-40-17.9-40-40l0-80zm40-8c-4.4 0-8 3.6-8 8l0 80c0 4.4 3.6 8 8 8l16 0c4.4 0 8-3.6 8-8l0-80c0-4.4-3.6-8-8-8l-16 0z";
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
    exports.faFileMov = exports.definition;
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

// src/_entries/faFileMov.js
var import_faFileMov = __toESM(require_faFileMov());
var export_faFileMov = import_faFileMov.definition;
export {
  export_faFileMov as faFileMov
};
