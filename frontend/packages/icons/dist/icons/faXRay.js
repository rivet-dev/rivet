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

// src/node_modules/@fortawesome/free-solid-svg-icons/faXRay.js
var require_faXRay = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faXRay.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "x-ray";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f497";
    var svgPathData = "M0 64C0 46.3 14.3 32 32 32l448 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l0 320c17.7 0 32 14.3 32 32s-14.3 32-32 32L32 480c-17.7 0-32-14.3-32-32s14.3-32 32-32L32 96C14.3 96 0 81.7 0 64zm280 56c0-13.3-10.7-24-24-24s-24 10.7-24 24l0 16-64 0c-13.3 0-24 10.7-24 24s10.7 24 24 24l64 0 0 48-80 0c-13.3 0-24 10.7-24 24s10.7 24 24 24l80 0 0 48-64 0c-13.3 0-24 10.7-24 24s10.7 24 24 24l64 0 0 16c0 13.3 10.7 24 24 24s24-10.7 24-24l0-16 64 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-64 0 0-48 80 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-80 0 0-48 64 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-64 0 0-16z";
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
    exports.faXRay = exports.definition;
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

// src/_entries/faXRay.js
var import_faXRay = __toESM(require_faXRay());
var export_faXRay = import_faXRay.definition;
export {
  export_faXRay as faXRay
};
