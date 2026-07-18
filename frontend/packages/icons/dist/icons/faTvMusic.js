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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTvMusic.js
var require_faTvMusic = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTvMusic.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "tv-music";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f8e6";
    var svgPathData = "M576 64l0 288L64 352 64 64l512 0zM64 0C28.7 0 0 28.7 0 64L0 352c0 35.3 28.7 64 64 64l512 0c35.3 0 64-28.7 64-64l0-288c0-35.3-28.7-64-64-64L64 0zM96 480c0 17.7 14.3 32 32 32l384 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-384 0c-17.7 0-32 14.3-32 32zM416 112c0-5.2-2.6-10.2-6.9-13.2s-9.8-3.7-14.7-1.8l-128 48c-6.2 2.3-10.4 8.3-10.4 15l0 48 0 49.8c-5-1.2-10.4-1.8-16-1.8c-26.5 0-48 14.3-48 32s21.5 32 48 32s48-14.3 48-32l0-68.9 96-36 0 42.7c-5-1.2-10.4-1.8-16-1.8c-26.5 0-48 14.3-48 32s21.5 32 48 32s48-14.3 48-32l0-96 0-48z";
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
    exports.faTvMusic = exports.definition;
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

// src/_entries/faTvMusic.js
var import_faTvMusic = __toESM(require_faTvMusic());
var export_faTvMusic = import_faTvMusic.definition;
export {
  export_faTvMusic as faTvMusic
};
