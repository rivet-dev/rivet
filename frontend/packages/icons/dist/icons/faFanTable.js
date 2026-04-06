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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFanTable.js
var require_faFanTable = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFanTable.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "fan-table";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e004";
    var svgPathData = "M256 413.6C355.7 398.2 432 312 432 208C432 93.1 338.9 0 224 0S16 93.1 16 208c0 104 76.3 190.2 176 205.6l0 34.4-96 0c-17.7 0-32 14.3-32 32s14.3 32 32 32l128 0 128 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-96 0 0-34.4zM184.9 68.5L224.3 166c18.5-25.3 50-39.8 83.3-35.1c48.1 6.8 81.7 51.3 74.9 99.4c-1.2 8.8-9.3 14.8-18.1 13.6L260.2 229.2c12.6 28.7 9.5 63.2-11.2 89.7c-29.9 38.3-85.2 45.1-123.5 15.2c-7-5.4-8.2-15.5-2.8-22.5l64.8-82.9c-31.2-3.4-59.5-23.4-72.1-54.5C97.2 129.1 119 77.9 164.1 59.7c8.2-3.3 17.5 .6 20.8 8.8zM224 224a16 16 0 1 0 0-32 16 16 0 1 0 0 32z";
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
    exports.faFanTable = exports.definition;
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

// src/_entries/faFanTable.js
var import_faFanTable = __toESM(require_faFanTable());
var export_faFanTable = import_faFanTable.definition;
export {
  export_faFanTable as faFanTable
};
