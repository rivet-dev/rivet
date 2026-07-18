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

// src/node_modules/@fortawesome/free-solid-svg-icons/faTableCellsColumnLock.js
var require_faTableCellsColumnLock = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faTableCellsColumnLock.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "table-cells-column-lock";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e678";
    var svgPathData = "M256 224l0 192 64 0 0-192-64 0zm256-79c-46.7 5.8-85.6 36.8-102.7 79l-25.3 0 0 90.8c-10.1 15.2-16 33.4-16 53.1l0 96c0 5.5 .5 10.9 1.3 16.1L128 480c-35.3 0-64-28.7-64-64L64 96c0-35.3 28.7-64 64-64l320 0c35.3 0 64 28.7 64 64l0 49zM128 224l0 192 64 0 0-192-64 0zm432 48.1c0-17.7-14.3-32-32-32s-32 14.3-32 32l0 47.9 64 0 0-47.9zM416 368c0-20.9 13.4-38.7 32-45.3l0-50.6c0-44.2 35.8-80 80-80s80 35.8 80 80l0 50.6c18.6 6.6 32 24.4 32 45.3l0 96c0 26.5-21.5 48-48 48l-128 0c-26.5 0-48-21.5-48-48l0-96z";
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
    exports.faTableCellsColumnLock = exports.definition;
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

// src/_entries/faTableCellsColumnLock.js
var import_faTableCellsColumnLock = __toESM(require_faTableCellsColumnLock());
var export_faTableCellsColumnLock = import_faTableCellsColumnLock.definition;
export {
  export_faTableCellsColumnLock as faTableCellsColumnLock
};
