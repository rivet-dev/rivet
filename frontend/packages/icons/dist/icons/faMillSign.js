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

// src/node_modules/@fortawesome/free-solid-svg-icons/faMillSign.js
var require_faMillSign = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faMillSign.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "mill-sign";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "e1ed";
    var svgPathData = "M297-22.2c12.3 5 18.2 19 13.2 31.3l-29 71.3C338.8 85.1 384 133.3 384 192l0 208c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-208c0-26.5-21.5-48-48-48-6.4 0-12.5 1.2-18 3.5l-30 73.8 0 178.7c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-21.2-57.8 142.2c-5 12.3-19 18.2-31.3 13.2s-18.2-19-13.2-31.3L160 251.3 160 192c0-26.5-21.5-48-48-48s-48 21.5-48 48l0 208c0 17.7-14.3 32-32 32S0 417.7 0 400L0 112c0-17.7 14.3-32 32-32 10.9 0 20.5 5.4 26.3 13.7 16-8.7 34.3-13.7 53.7-13.7 31.3 0 59.7 12.9 80 33.6 9.6-9.8 20.9-17.8 33.5-23.5L265.8-9c5-12.3 19-18.2 31.3-13.2z";
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
    exports.faMillSign = exports.definition;
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

// src/_entries/faMillSign.js
var import_faMillSign = __toESM(require_faMillSign());
var export_faMillSign = import_faMillSign.definition;
export {
  export_faMillSign as faMillSign
};
