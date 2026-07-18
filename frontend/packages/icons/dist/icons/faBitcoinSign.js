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

// src/node_modules/@fortawesome/free-solid-svg-icons/faBitcoinSign.js
var require_faBitcoinSign = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faBitcoinSign.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bitcoin-sign";
    var width = 320;
    var height = 512;
    var aliases = [];
    var unicode = "e0b4";
    var svgPathData = "M64 24C64 10.7 74.7 0 88 0s24 10.7 24 24l0 40 32 0 0-40c0-13.3 10.7-24 24-24s24 10.7 24 24l0 41.1c54.3 7.8 96 54.4 96 110.9 0 24.2-7.7 46.6-20.7 64.9 31.7 19.8 52.7 55 52.7 95.1 0 61.9-50.1 112-112 112l-16 0 0 40c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-40-32 0 0 40c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-40-22.3 0C18.7 448 0 429.3 0 406.3L0 101.6C0 80.8 16.8 64 37.6 64L64 64 64 24zm0 200l112 0c26.5 0 48-21.5 48-48s-21.5-48-48-48l-112 0 0 96zm112 64l-112 0 0 96 144 0c26.5 0 48-21.5 48-48s-21.5-48-48-48l-32 0z";
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
    exports.faBitcoinSign = exports.definition;
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

// src/_entries/faBitcoinSign.js
var import_faBitcoinSign = __toESM(require_faBitcoinSign());
var export_faBitcoinSign = import_faBitcoinSign.definition;
export {
  export_faBitcoinSign as faBitcoinSign
};
