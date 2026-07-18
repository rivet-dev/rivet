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

// src/node_modules/@fortawesome/pro-solid-svg-icons/fa360Degrees.js
var require_fa360Degrees = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/fa360Degrees.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "360-degrees";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e2dc";
    var svgPathData = "M640 32a32 32 0 1 0 -64 0 32 32 0 1 0 64 0zM32 64C14.3 64 0 78.3 0 96s14.3 32 32 32l64 0L38.4 204.8c-7.3 9.7-8.4 22.7-3 33.5S51.9 256 64 256l16 0c26.5 0 48 21.5 48 48l0 48c0 17.7-14.3 32-32 32l-5.5 0c-8.5 0-16.6-3.4-22.6-9.4L54.6 361.4c-12.5-12.5-32.8-12.5-45.3 0s-12.5 32.8 0 45.3l13.3 13.3c18 18 42.4 28.1 67.9 28.1l5.5 0c53 0 96-43 96-96l0-48c0-47-29-87.3-70.1-103.9l63.7-84.9c7.3-9.7 8.4-22.7 3-33.5S172.1 64 160 64L32 64zm448 80c0-8.8 7.2-16 16-16s16 7.2 16 16l0 224c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-224zm96 224l0-224c0-44.2-35.8-80-80-80s-80 35.8-80 80l0 224c0 44.2 35.8 80 80 80s80-35.8 80-80zM288 160c0-17.7 14.3-32 32-32s32-14.3 32-32s-14.3-32-32-32c-53 0-96 43-96 96l0 128 0 .2 0 79.8c0 44.2 35.8 80 80 80s80-35.8 80-80l0-96c0-44.2-35.8-80-80-80c-5.5 0-10.8 .6-16 1.6l0-33.6zm0 112c0-8.8 7.2-16 16-16s16 7.2 16 16l0 96c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-96z";
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
    exports.fa360Degrees = exports.definition;
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

// src/_entries/fa360Degrees.js
var import_fa360Degrees = __toESM(require_fa360Degrees());
var export_fa360Degrees = import_fa360Degrees.definition;
export {
  export_fa360Degrees as fa360Degrees
};
