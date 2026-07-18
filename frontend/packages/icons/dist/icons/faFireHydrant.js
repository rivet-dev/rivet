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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFireHydrant.js
var require_faFireHydrant = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFireHydrant.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "fire-hydrant";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "e17f";
    var svgPathData = "M224 32l0 4c45 11.6 80.4 47 92 92l12 0c13.3 0 24 10.7 24 24s-10.7 24-24 24L56 176c-13.3 0-24-10.7-24-24s10.7-24 24-24l12 0c11.6-45 47-80.4 92-92l0-4c0-17.7 14.3-32 32-32s32 14.3 32 32zM32 488c0-13.3 10.7-24 24-24l8 0 0-64-16 0c-17.7 0-32-14.3-32-32l0-64c0-17.7 14.3-32 32-32l16 0 0-64 256 0 0 64 16 0c17.7 0 32 14.3 32 32l0 64c0 17.7-14.3 32-32 32l-16 0 0 64 8 0c13.3 0 24 10.7 24 24s-10.7 24-24 24l-8 0L64 512l-8 0c-13.3 0-24-10.7-24-24zm160-88a64 64 0 1 0 0-128 64 64 0 1 0 0 128z";
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
    exports.faFireHydrant = exports.definition;
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

// src/_entries/faFireHydrant.js
var import_faFireHydrant = __toESM(require_faFireHydrant());
var export_faFireHydrant = import_faFireHydrant.definition;
export {
  export_faFireHydrant as faFireHydrant
};
