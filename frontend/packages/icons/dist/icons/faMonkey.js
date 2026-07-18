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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faMonkey.js
var require_faMonkey = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faMonkey.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "monkey";
    var width = 576;
    var height = 512;
    var aliases = [128018];
    var unicode = "f6fb";
    var svgPathData = "M160 416c0-89.7 61.4-165 144.5-186.1C329.1 265 369.9 288 416 288c9.8 0 19.4-1 28.7-3L526.5 363c31.6 30.2 49.5 72 49.5 115.7l0 1.2c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-1.2c0-26.2-10.7-51.3-29.7-69.4L416 346.1 416 448l32 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-32 0-256 0C71.6 512 0 440.4 0 352L0 110.1C0 49.3 49.3 0 110.1 0l10.4 0c57.2 0 102.8 47.8 100.1 104.9c-2.3 48.2-38.6 88-86.4 94.6l-1.9 .3c-17.5 2.4-33.7-9.8-36.1-27.3s9.8-33.7 27.3-36.1l1.9-.3c17.3-2.4 30.4-16.7 31.2-34.2c1-20.6-15.5-37.9-36.2-37.9l-10.4 0C84.7 64 64 84.7 64 110.1L64 352c0 53 43 96 96 96l0-32zM416 256c-29.3 0-55.9-12.2-74.8-31.7c-9-9.3-16.2-20.2-21.3-32.3c-35.3 0-64-28.7-64-64s28.6-64 64-64c15.7-37.6 52.8-64 96-64s80.4 26.4 96 64c35.3 0 64 28.7 64 64s-28.6 64-64 64c-15.7 37.6-52.8 64-96 64zM384 64c-26.5 0-48 21.5-48 48c0 20.8 13.2 38.4 31.6 45.1c4.7 1.7 8.4 5.9 8.4 10.9c0 22.1 17.9 40 40 40s40-17.9 40-40c0-5 3.7-9.2 8.4-10.9c18.5-6.7 31.6-24.4 31.6-45.1c0-26.5-21.5-48-48-48l-64 0zm0 32a16 16 0 1 1 0 32 16 16 0 1 1 0-32zm48 16a16 16 0 1 1 32 0 16 16 0 1 1 -32 0z";
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
    exports.faMonkey = exports.definition;
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

// src/_entries/faMonkey.js
var import_faMonkey = __toESM(require_faMonkey());
var export_faMonkey = import_faMonkey.definition;
export {
  export_faMonkey as faMonkey
};
