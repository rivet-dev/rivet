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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBagSeedling.js
var require_faBagSeedling = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBagSeedling.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bag-seedling";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e5f2";
    var svgPathData = "M64 64L38.6 38.6c-4.2-4.2-6.6-10-6.6-16C32 10.1 42.1 0 54.6 0L457.4 0C469.9 0 480 10.1 480 22.6c0 6-2.4 11.8-6.6 16L448 64 64 64zM56 96l400 0 10.6 42.5c8.9 35.5 13.4 72 13.4 108.7l0 17.7c0 36.6-4.5 73.1-13.4 108.7L456 416 56 416 45.4 373.5C36.5 338 32 301.5 32 264.8l0-17.7c0-36.6 4.5-73.1 13.4-108.7L56 96zm8 352l384 0 25.4 25.4c4.2 4.2 6.6 10 6.6 16c0 12.5-10.1 22.6-22.6 22.6L54.6 512C42.1 512 32 501.9 32 489.4c0-6 2.4-11.8 6.6-16L64 448zm80-288c-8.8 0-16 7.2-16 16c0 59.2 45.9 107.6 104 111.7l0 40.3c0 13.3 10.7 24 24 24s24-10.7 24-24l0-40.3c58.1-4.1 104-52.6 104-111.7c0-8.8-7.2-16-16-16l-2 0c-47.3 0-88.4 26-110 64.5C234.4 186 193.3 160 146 160l-2 0z";
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
    exports.faBagSeedling = exports.definition;
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

// src/_entries/faBagSeedling.js
var import_faBagSeedling = __toESM(require_faBagSeedling());
var export_faBagSeedling = import_faBagSeedling.definition;
export {
  export_faBagSeedling as faBagSeedling
};
