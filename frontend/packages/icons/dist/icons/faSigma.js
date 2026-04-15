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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSigma.js
var require_faSigma = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSigma.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "sigma";
    var width = 384;
    var height = 512;
    var aliases = [8721];
    var unicode = "f68b";
    var svgPathData = "M2.4 51.8C7.4 39.8 19.1 32 32 32l288 0c35.3 0 64 28.7 64 64l0 32c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-32L109.3 96 246.6 233.4c12.5 12.5 12.5 32.8 0 45.3L109.3 416 320 416l0-32c0-17.7 14.3-32 32-32s32 14.3 32 32l0 32c0 35.3-28.7 64-64 64L32 480c-12.9 0-24.6-7.8-29.6-19.8s-2.2-25.7 6.9-34.9L178.7 256 9.4 86.6C.2 77.5-2.5 63.7 2.4 51.8z";
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
    exports.faSigma = exports.definition;
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

// src/_entries/faSigma.js
var import_faSigma = __toESM(require_faSigma());
var export_faSigma = import_faSigma.definition;
export {
  export_faSigma as faSigma
};
