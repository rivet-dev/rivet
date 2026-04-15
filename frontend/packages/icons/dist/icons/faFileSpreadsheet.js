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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFileSpreadsheet.js
var require_faFileSpreadsheet = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFileSpreadsheet.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "file-spreadsheet";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "f65b";
    var svgPathData = "M64 0C28.7 0 0 28.7 0 64L0 448c0 35.3 28.7 64 64 64l256 0c35.3 0 64-28.7 64-64l0-288-128 0c-17.7 0-32-14.3-32-32L224 0 64 0zM256 0l0 128 128 0L256 0zM88 224l208 0c17.7 0 32 14.3 32 32l0 16 0 80 0 64c0 17.7-14.3 32-32 32l-64 0-80 0-64 0c-17.7 0-32-14.3-32-32l0-64 0-80 0-16c0-17.7 14.3-32 32-32zm0 112l48 0 0-48-48 0 0 48zm80 0l48 0 0-48-48 0 0 48zm80 0l48 0 0-48-48 0 0 48zm0 32l0 48 48 0 0-48-48 0zm-32 0l-48 0 0 48 48 0 0-48zm-80 0l-48 0 0 48 48 0 0-48z";
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
    exports.faFileSpreadsheet = exports.definition;
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

// src/_entries/faFileSpreadsheet.js
var import_faFileSpreadsheet = __toESM(require_faFileSpreadsheet());
var export_faFileSpreadsheet = import_faFileSpreadsheet.definition;
export {
  export_faFileSpreadsheet as faFileSpreadsheet
};
