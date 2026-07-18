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

// src/node_modules/@fortawesome/free-solid-svg-icons/faCentSign.js
var require_faCentSign = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faCentSign.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "cent-sign";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "e3f5";
    var svgPathData = "M208 0c17.7 0 32 14.3 32 32l0 25.4c43.6 5.2 83 24.5 113.3 53.1 12.9 12.1 13.4 32.4 1.3 45.2s-32.4 13.4-45.2 1.3c-24.4-23-57.2-37.1-93.3-37.1-75.1 0-136 60.9-136 136s60.9 136 136 136c36.2 0 69-14.1 93.3-37.1 12.9-12.1 33.1-11.5 45.2 1.3s11.5 33.1-1.3 45.2C323 430.1 283.6 449.4 240 454.6l0 25.4c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-28C84.7 433.5 16 352.8 16 256S84.7 78.5 176 60l0-28c0-17.7 14.3-32 32-32z";
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
    exports.faCentSign = exports.definition;
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

// src/_entries/faCentSign.js
var import_faCentSign = __toESM(require_faCentSign());
var export_faCentSign = import_faCentSign.definition;
export {
  export_faCentSign as faCentSign
};
