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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFilesMedical.js
var require_faFilesMedical = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFilesMedical.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "files-medical";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f7fd";
    var svgPathData = "M160 0L320 0l0 96c0 17.7 14.3 32 32 32l96 0 0 224c0 35.3-28.7 64-64 64l-224 0c-35.3 0-64-28.7-64-64L96 64c0-35.3 28.7-64 64-64zM416 64l32 32-45.3 0L384 96l-32 0 0-32 0-18.7L352 0l32 32 32 32zM24 96c13.3 0 24 10.7 24 24l0 256c0 48.6 39.4 88 88 88l192 0c13.3 0 24 10.7 24 24s-10.7 24-24 24l-192 0C60.9 512 0 451.1 0 376L0 120c0-13.3 10.7-24 24-24zm216 80l0 48-48 0c-8.8 0-16 7.2-16 16l0 32c0 8.8 7.2 16 16 16l48 0 0 48c0 8.8 7.2 16 16 16l32 0c8.8 0 16-7.2 16-16l0-48 48 0c8.8 0 16-7.2 16-16l0-32c0-8.8-7.2-16-16-16l-48 0 0-48c0-8.8-7.2-16-16-16l-32 0c-8.8 0-16 7.2-16 16z";
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
    exports.faFilesMedical = exports.definition;
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

// src/_entries/faFilesMedical.js
var import_faFilesMedical = __toESM(require_faFilesMedical());
var export_faFilesMedical = import_faFilesMedical.definition;
export {
  export_faFilesMedical as faFilesMedical
};
