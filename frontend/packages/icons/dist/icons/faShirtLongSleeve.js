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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faShirtLongSleeve.js
var require_faShirtLongSleeve = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faShirtLongSleeve.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "shirt-long-sleeve";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e3c7";
    var svgPathData = "M157 .3c-32.1 2.5-61 20.9-76.7 49.5L13.9 170.5C4.8 187 0 205.6 0 224.4L0 384c0 17.7 14.3 32 32 32l32 0c17.7 0 32-14.3 32-32l0-159.6c0-2.7 .7-5.3 2-7.7l46-83.7L144 448c0 35.3 28.7 64 64 64l224 0c35.3 0 64-28.7 64-64l0-314.9 46 83.7c1.3 2.4 2 5 2 7.7L544 384c0 17.7 14.3 32 32 32l32 0c17.7 0 32-14.3 32-32l0-159.6c0-18.9-4.8-37.4-13.9-54L559.7 49.7C544.1 21.2 515.1 2.8 483 .3c-1-.2-2-.3-3-.3l-4.4 0L460 0 432 0c-8.8 0-15.9 7.2-17.3 15.9C407.1 61.4 367.6 96 320 96s-87.1-34.6-94.7-80.1C223.9 7.2 216.8 0 208 0L180 0 164.4 0 160 0c-1 0-2 .1-3 .3z";
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
    exports.faShirtLongSleeve = exports.definition;
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

// src/_entries/faShirtLongSleeve.js
var import_faShirtLongSleeve = __toESM(require_faShirtLongSleeve());
var export_faShirtLongSleeve = import_faShirtLongSleeve.definition;
export {
  export_faShirtLongSleeve as faShirtLongSleeve
};
