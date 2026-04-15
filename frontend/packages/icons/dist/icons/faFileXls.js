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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFileXls.js
var require_faFileXls = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFileXls.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "file-xls";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e64d";
    var svgPathData = "M0 64C0 28.7 28.7 0 64 0L224 0l0 128c0 17.7 14.3 32 32 32l128 0 0 144-176 0c-35.3 0-64 28.7-64 64l0 144-80 0c-35.3 0-64-28.7-64-64L0 64zm384 64l-128 0L256 0 384 128zM224 368c0 7.3 2.2 14.4 6.2 20.4l9.8 14.7 9.8-14.7c4-6.1 6.2-13.2 6.2-20.4c0-8.8 7.2-16 16-16s16 7.2 16 16c0 13.6-4 26.9-11.6 38.2L259.2 432l17.2 25.8C284 469.1 288 482.4 288 496c0 8.8-7.2 16-16 16s-16-7.2-16-16c0-7.3-2.2-14.4-6.2-20.4L240 460.8l-9.8 14.7c-4 6.1-6.2 13.2-6.2 20.4c0 8.8-7.2 16-16 16s-16-7.2-16-16c0-13.6 4-26.9 11.6-38.2L220.8 432l-17.2-25.8C196 394.9 192 381.6 192 368c0-8.8 7.2-16 16-16s16 7.2 16 16zm96 128l0-128c0-8.8 7.2-16 16-16s16 7.2 16 16l0 112 32 0c8.8 0 16 7.2 16 16s-7.2 16-16 16l-48 0c-8.8 0-16-7.2-16-16zm88-98.3c0-25.2 20.4-45.7 45.7-45.7l26.3 0c8.8 0 16 7.2 16 16s-7.2 16-16 16l-26.3 0c-7.5 0-13.7 6.1-13.7 13.7c0 5.2 2.9 9.9 7.6 12.2l31.2 15.6c15.5 7.7 25.2 23.5 25.2 40.8c0 25.2-20.4 45.7-45.7 45.7L432 512c-8.8 0-16-7.2-16-16s7.2-16 16-16l26.3 0c7.5 0 13.7-6.1 13.7-13.7c0-5.2-2.9-9.9-7.6-12.2l-31.2-15.6C417.8 430.8 408 415 408 397.7z";
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
    exports.faFileXls = exports.definition;
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

// src/_entries/faFileXls.js
var import_faFileXls = __toESM(require_faFileXls());
var export_faFileXls = import_faFileXls.definition;
export {
  export_faFileXls as faFileXls
};
