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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFonduePot.js
var require_faFonduePot = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFonduePot.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "fondue-pot";
    var width = 512;
    var height = 512;
    var aliases = [129749];
    var unicode = "e40d";
    var svgPathData = "M319 39.8l-24 96c-2.9 11.5-11.6 20-22.2 23L264.5 192l-33 0 10.2-41c-7.9-7.7-11.7-19.3-8.8-30.8l24-96C261.2 7.1 278.6-3.3 295.8 1s27.6 21.7 23.3 38.8zm62.9 120.2L360.6 192l-38.5 0 33.2-49.8c-4.9-9.9-4.5-22.1 2.1-31.9l64-96C431.2-.5 451-4.4 465.8 5.4s18.7 29.7 8.9 44.4l-64 96c-6.6 9.9-17.7 14.9-28.7 14.2zM32 224l32 0 0 16 0 64c0 8.8 7.2 16 16 16s16-7.2 16-16l0-32c0-8.8 7.2-16 16-16s16 7.2 16 16s7.2 16 16 16s16-7.2 16-16l0-32 0-16 224 0 64 0 32 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-32 0 0 32c0 52.3-25.1 98.8-64 128l0 32c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-3.2c-10.3 2.1-21 3.2-32 3.2l-128 0c-11 0-21.7-1.1-32-3.2l0 3.2c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-32C25.1 418.8 0 372.3 0 320l0-64c0-17.7 14.3-32 32-32z";
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
    exports.faFonduePot = exports.definition;
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

// src/_entries/faFonduePot.js
var import_faFonduePot = __toESM(require_faFonduePot());
var export_faFonduePot = import_faFonduePot.definition;
export {
  export_faFonduePot as faFonduePot
};
