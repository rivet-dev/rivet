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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faArrowUpLeftFromCircle.js
var require_faArrowUpLeftFromCircle = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faArrowUpLeftFromCircle.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "arrow-up-left-from-circle";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e09e";
    var svgPathData = "M192 0L32 0C14.3 0 0 14.3 0 32L0 192c0 17.7 14.3 32 32 32s32-14.3 32-32l0-82.7L297.4 342.6c12.5 12.5 32.8 12.5 45.3 0s12.5-32.8 0-45.3L109.3 64 192 64c17.7 0 32-14.3 32-32s-14.3-32-32-32zM325.3 67.1c-17.4-2.9-33.9 8.8-36.9 26.3s8.8 33.9 26.3 36.9C390.4 142.9 448 208.8 448 288c0 88.4-71.6 160-160 160c-79.2 0-145.1-57.6-157.8-133.3c-2.9-17.4-19.4-29.2-36.9-26.3s-29.2 19.4-26.3 36.9C84.9 431.3 177 512 288 512c123.7 0 224-100.3 224-224c0-111-80.7-203.1-186.7-220.9z";
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
    exports.faArrowUpLeftFromCircle = exports.definition;
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

// src/_entries/faArrowUpLeftFromCircle.js
var import_faArrowUpLeftFromCircle = __toESM(require_faArrowUpLeftFromCircle());
var export_faArrowUpLeftFromCircle = import_faArrowUpLeftFromCircle.definition;
export {
  export_faArrowUpLeftFromCircle as faArrowUpLeftFromCircle
};
