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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCassetteVhs.js
var require_faCassetteVhs = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCassetteVhs.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "cassette-vhs";
    var width = 576;
    var height = 512;
    var aliases = [128252, "vhs"];
    var unicode = "f8ec";
    var svgPathData = "M0 96l0 32 576 0 0-32c0-35.3-28.7-64-64-64L64 32C28.7 32 0 60.7 0 96zm0 64L0 416c0 35.3 28.7 64 64 64l448 0c35.3 0 64-28.7 64-64l0-256L0 160zM467.9 384L416 384l0-34.7c18.6-6.6 32-24.4 32-45.3s-13.4-38.7-32-45.3l0-34.7 51.9 0c17.6 21.9 28.1 49.7 28.1 80s-10.5 58.1-28.1 80zM160 349.3l0 34.7-51.9 0C90.5 362.1 80 334.3 80 304s10.5-58.1 28.1-80l51.9 0 0 34.7c-18.6 6.6-32 24.4-32 45.3s13.4 38.7 32 45.3zM256 224l64 0c17.7 0 32 14.3 32 32l0 96c0 17.7-14.3 32-32 32l-64 0c-17.7 0-32-14.3-32-32l0-96c0-17.7 14.3-32 32-32z";
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
    exports.faCassetteVhs = exports.definition;
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

// src/_entries/faVhs.js
var import_faCassetteVhs = __toESM(require_faCassetteVhs());
var export_faVhs = import_faCassetteVhs.definition;
export {
  export_faVhs as faVhs
};
