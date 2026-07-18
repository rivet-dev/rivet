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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSpinnerScale.js
var require_faSpinnerScale = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSpinnerScale.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "spinner-scale";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e62a";
    var svgPathData = "M256 116a52 52 0 1 1 0-104 52 52 0 1 1 0 104zm0 364a32 32 0 1 1 0-64 32 32 0 1 1 0 64zM448 288a32 32 0 1 1 0-64 32 32 0 1 1 0 64zM32 256a32 32 0 1 1 64 0 32 32 0 1 1 -64 0zm399.4-96.2A56 56 0 1 1 352.2 80.6a56 56 0 1 1 79.2 79.2zM97.6 414.4a32 32 0 1 1 45.3-45.3A32 32 0 1 1 97.6 414.4zm271.5 0a32 32 0 1 1 45.3-45.3 32 32 0 1 1 -45.3 45.3zM86.3 86.3a48 48 0 1 1 67.9 67.9A48 48 0 1 1 86.3 86.3z";
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
    exports.faSpinnerScale = exports.definition;
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

// src/_entries/faSpinnerScale.js
var import_faSpinnerScale = __toESM(require_faSpinnerScale());
var export_faSpinnerScale = import_faSpinnerScale.definition;
export {
  export_faSpinnerScale as faSpinnerScale
};
