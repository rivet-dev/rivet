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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFishingRod.js
var require_faFishingRod = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFishingRod.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "fishing-rod";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e3a8";
    var svgPathData = "M295.9 48c-3 0-6 .4-8.8 1.2L209.8 71.3C137.7 91.9 88 157.8 88 232.8L88 288l8 0c17.7 0 32 14.3 32 32l0 160c0 17.7-14.3 32-32 32l-64 0c-17.7 0-32-14.3-32-32L0 320c0-17.7 14.3-32 32-32l8 0 0-55.2c0-96.4 63.9-181.2 156.7-207.7L273.8 3.1C281 1 288.4 0 295.9 0C340.1 0 376 35.9 376 80.1l0 55.9c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-55.9C328 62.4 313.6 48 295.9 48zM256 344c0-71.6 55.6-127.8 89-148.1c4.3-2.6 9.6-2.6 14 0c33.5 20.3 89 76.6 89 148.1c0 32-16 80-64 112l27.3 27.3c3 3 4.7 7.1 4.7 11.3l0 1.4c0 8.8-7.2 16-16 16l-96 0c-8.8 0-16-7.2-16-16l0-1.4c0-4.2 1.7-8.3 4.7-11.3L320 456c-48-32-64-80-64-112zm128-32a24 24 0 1 0 -48 0 24 24 0 1 0 48 0z";
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
    exports.faFishingRod = exports.definition;
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

// src/_entries/faFishingRod.js
var import_faFishingRod = __toESM(require_faFishingRod());
var export_faFishingRod = import_faFishingRod.definition;
export {
  export_faFishingRod as faFishingRod
};
