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

// src/node_modules/@fortawesome/free-solid-svg-icons/faSquareEnvelope.js
var require_faSquareEnvelope = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faSquareEnvelope.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "square-envelope";
    var width = 448;
    var height = 512;
    var aliases = ["envelope-square"];
    var unicode = "f199";
    var svgPathData = "M64 32C28.7 32 0 60.7 0 96L0 416c0 35.3 28.7 64 64 64l320 0c35.3 0 64-28.7 64-64l0-320c0-35.3-28.7-64-64-64L64 32zM209.1 267.9L108.4 207.4c-7.7-4.6-12.4-12.9-12.4-21.9 0-14.1 11.4-25.5 25.5-25.5l204.9 0c14.1 0 25.5 11.4 25.5 25.5 0 9-4.7 17.3-12.4 21.9L238.9 267.9c-4.5 2.7-9.6 4.1-14.9 4.1s-10.4-1.4-14.9-4.1zM352 237.3l0 82.7c0 17.7-14.3 32-32 32l-192 0c-17.7 0-32-14.3-32-32l0-82.7 96.7 58C202.1 301 213 304 224 304s21.9-3 31.3-8.7l96.7-58z";
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
    exports.faSquareEnvelope = exports.definition;
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

// src/_entries/faEnvelopeSquare.js
var import_faSquareEnvelope = __toESM(require_faSquareEnvelope());
var export_faEnvelopeSquare = import_faSquareEnvelope.definition;
export {
  export_faEnvelopeSquare as faEnvelopeSquare
};
