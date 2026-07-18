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

// src/node_modules/@fortawesome/free-solid-svg-icons/faPumpSoap.js
var require_faPumpSoap = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faPumpSoap.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "pump-soap";
    var width = 320;
    var height = 512;
    var aliases = [];
    var unicode = "e06b";
    var svgPathData = "M96 0l0 80 128 0 0-24 72 0c13.3 0 24-10.7 24-24S309.3 8 296 8l-72 0 0-8c0-17.7-14.3-32-32-32l-64 0C110.3-32 96-17.7 96 0zM64 128c-35.3 0-64 28.7-64 64L0 448c0 35.3 28.7 64 64 64l192 0c35.3 0 64-28.7 64-64l0-256c0-35.3-28.7-64-64-64L64 128zM224 344c0 35.3-28.7 56-64 56s-64-20.7-64-56c0-32.5 37-80.9 50.9-97.9 3.2-3.9 8.1-6.1 13.1-6.1s9.9 2.2 13.1 6.1C187 263.1 224 311.5 224 344z";
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
    exports.faPumpSoap = exports.definition;
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

// src/_entries/faPumpSoap.js
var import_faPumpSoap = __toESM(require_faPumpSoap());
var export_faPumpSoap = import_faPumpSoap.definition;
export {
  export_faPumpSoap as faPumpSoap
};
