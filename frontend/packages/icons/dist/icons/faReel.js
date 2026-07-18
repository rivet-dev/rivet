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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faReel.js
var require_faReel = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faReel.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "reel";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e238";
    var svgPathData = "M0 32C0 14.3 14.3 0 32 0L416 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-32 0 0 24c0 13.3-10.7 24-24 24L88 112c-13.3 0-24-10.7-24-24l0-24L32 64C14.3 64 0 49.7 0 32zM0 480c0-17.7 14.3-32 32-32l32 0 0-24c0-13.3 10.7-24 24-24l272 0c13.3 0 24 10.7 24 24l0 24 32 0c17.7 0 32 14.3 32 32s-14.3 32-32 32L32 512c-17.7 0-32-14.3-32-32zM80 144l288 0c8.8 0 16 7.2 16 16s-7.2 16-16 16L80 176c-8.8 0-16-7.2-16-16s7.2-16 16-16zm0 64l352 0c8.8 0 16 7.2 16 16s-7.2 16-16 16L80 240c-8.8 0-16-7.2-16-16s7.2-16 16-16zm0 64l288 0c8.8 0 16 7.2 16 16s-7.2 16-16 16L80 304c-8.8 0-16-7.2-16-16s7.2-16 16-16zm0 64l288 0c8.8 0 16 7.2 16 16s-7.2 16-16 16L80 368c-8.8 0-16-7.2-16-16s7.2-16 16-16z";
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
    exports.faReel = exports.definition;
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

// src/_entries/faReel.js
var import_faReel = __toESM(require_faReel());
var export_faReel = import_faReel.definition;
export {
  export_faReel as faReel
};
