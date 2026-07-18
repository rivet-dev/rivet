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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faVideoArrowDownLeft.js
var require_faVideoArrowDownLeft = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faVideoArrowDownLeft.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "video-arrow-down-left";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e2c8";
    var svgPathData = "M64 64C28.7 64 0 92.7 0 128L0 384c0 35.3 28.7 64 64 64l256 0c35.3 0 64-28.7 64-64l0-256c0-35.3-28.7-64-64-64L64 64zm512 64c0-11.8-6.5-22.6-16.9-28.2s-23-5-32.9 1.6l-96 64L416 174.9l0 17.1 0 128 0 17.1 14.2 9.5 96 64c9.8 6.5 22.4 7.2 32.9 1.6s16.9-16.4 16.9-28.2l0-256zM232 352l-112 0c-13.3 0-24-10.7-24-24l0-112c0-13.3 10.7-24 24-24s24 10.7 24 24l0 54.1L247 167c9.4-9.4 24.6-9.4 33.9 0s9.4 24.6 0 33.9l-103 103 54.1 0c13.3 0 24 10.7 24 24s-10.7 24-24 24z";
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
    exports.faVideoArrowDownLeft = exports.definition;
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

// src/_entries/faVideoArrowDownLeft.js
var import_faVideoArrowDownLeft = __toESM(require_faVideoArrowDownLeft());
var export_faVideoArrowDownLeft = import_faVideoArrowDownLeft.definition;
export {
  export_faVideoArrowDownLeft as faVideoArrowDownLeft
};
