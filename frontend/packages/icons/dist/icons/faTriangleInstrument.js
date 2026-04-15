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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTriangleInstrument.js
var require_faTriangleInstrument = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTriangleInstrument.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "triangle-instrument";
    var width = 576;
    var height = 512;
    var aliases = ["triangle-music"];
    var unicode = "f8e2";
    var svgPathData = "M320 32c0-17.7-14.3-32-32-32s-32 14.3-32 32l0 37.9c-18 6.8-33.6 19.4-44.1 36.1L46.5 367.7C37 382.8 32 400.2 32 418c0 51.9 42.1 94 94 94L450 512c51.9 0 94-42.1 94-94c0-17.8-5-35.2-14.5-50.2L448.2 239l-46.4 46.4 73.6 116.5c3 4.8 4.6 10.4 4.6 16c0 16.6-13.5 30-30 30L126 448c-16.6 0-30-13.5-30-30c0-5.7 1.6-11.2 4.6-16L266 140.1c4.8-7.6 13.1-12.1 22-12.1s17.3 4.6 22 12.1l48 75.9-51.2 51.2c-5.9-2.1-12.2-3.2-18.8-3.2c-30.9 0-56 25.1-56 56s25.1 56 56 56s56-25.1 56-56c0-6.6-1.1-12.9-3.2-18.8l43.5-43.5 46.4-46.4L537 105c9.4-9.4 9.4-24.6 0-33.9s-24.6-9.4-33.9 0l-98.6 98.6L364.1 106C353.6 89.2 338 76.7 320 69.9L320 32z";
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
    exports.faTriangleInstrument = exports.definition;
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

// src/_entries/faTriangleInstrument.js
var import_faTriangleInstrument = __toESM(require_faTriangleInstrument());
var export_faTriangleInstrument = import_faTriangleInstrument.definition;
export {
  export_faTriangleInstrument as faTriangleInstrument
};
