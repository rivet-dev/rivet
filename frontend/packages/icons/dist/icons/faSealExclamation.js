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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSealExclamation.js
var require_faSealExclamation = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSealExclamation.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "seal-exclamation";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e242";
    var svgPathData = "M222.1 17.9c18.7-18.7 49.1-18.7 67.9 0L336 64l64 0c26.5 0 48 21.5 48 48l0 64 45.6 45.6c18.7 18.7 18.7 49.1 0 67.9L448 335.1l0 64.9c0 26.5-21.5 48-48 48l-64.9 0-45.1 45.1c-18.7 18.7-49.1 18.7-67.9 0L176.9 448 112 448c-26.5 0-48-21.5-48-48l0-64.9L18.4 289.5c-18.7-18.7-18.7-49.1 0-67.9L64 176l0-64c0-26.5 21.5-48 48-48l64 0 46.1-46.1zM256 128c-13.3 0-24 10.7-24 24l0 112c0 13.3 10.7 24 24 24s24-10.7 24-24l0-112c0-13.3-10.7-24-24-24zm32 224a32 32 0 1 0 -64 0 32 32 0 1 0 64 0z";
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
    exports.faSealExclamation = exports.definition;
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

// src/_entries/faSealExclamation.js
var import_faSealExclamation = __toESM(require_faSealExclamation());
var export_faSealExclamation = import_faSealExclamation.definition;
export {
  export_faSealExclamation as faSealExclamation
};
