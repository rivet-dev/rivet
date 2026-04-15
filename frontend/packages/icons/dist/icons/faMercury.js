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

// src/node_modules/@fortawesome/free-solid-svg-icons/faMercury.js
var require_faMercury = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faMercury.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "mercury";
    var width = 384;
    var height = 512;
    var aliases = [9791];
    var unicode = "f223";
    var svgPathData = "M117-4C106-17.8 85.8-20 72.1-9s-16 31.2-5 45C74.9 45.7 83.7 54.5 93.5 62.1 46.7 93.7 16 147.3 16 208 16 294.3 78.1 366.1 160 381.1l0 34.9-32 0c-17.7 0-32 14.3-32 32s14.3 32 32 32l32 0 0 32c0 17.7 14.3 32 32 32s32-14.3 32-32l0-32 32 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-32 0 0-34.9c81.9-15 144-86.8 144-173.1 0-60.7-30.7-114.3-77.5-145.9 9.8-7.7 18.7-16.4 26.4-26.1 11.1-13.8 8.8-33.9-5-45S278-17.8 267-4c-17.6 22-44.7 36-75 36S134.7 18 117-4zm75 324c-61.9 0-112-50.1-112-112S130.1 96 192 96 304 146.1 304 208c0 61.8-50.1 112-111.9 112l-.1 0z";
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
    exports.faMercury = exports.definition;
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

// src/_entries/faMercury.js
var import_faMercury = __toESM(require_faMercury());
var export_faMercury = import_faMercury.definition;
export {
  export_faMercury as faMercury
};
