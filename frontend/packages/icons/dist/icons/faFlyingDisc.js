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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFlyingDisc.js
var require_faFlyingDisc = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFlyingDisc.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "flying-disc";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e3a9";
    var svgPathData = "M23.7 456.3C-1.5 431.1-5.9 389.6 7.1 341.4c3.2 9.5 8.4 18.5 16.2 26.3c15.6 15.6 36 20.7 55 20.5c18.8-.1 38.8-5.3 58.2-13.3c39-16.1 82-46 121.2-85.1s69-82.2 85.1-121.2c8-19.4 13.2-39.4 13.3-58.2c.1-19-5-39.5-20.5-55c-7.8-7.8-16.8-13-26.3-16.2c48.2-13 89.8-8.6 114.9 16.6c55.3 55.3 10.5 189.8-100.2 300.5S79 511.6 23.7 456.3zM235.1 267.1C161.4 340.9 76.7 375.8 46 345s4.1-115.4 77.9-189.2S282.3 47.2 313 77.9s-4.1 115.4-77.9 189.2z";
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
    exports.faFlyingDisc = exports.definition;
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

// src/_entries/faFlyingDisc.js
var import_faFlyingDisc = __toESM(require_faFlyingDisc());
var export_faFlyingDisc = import_faFlyingDisc.definition;
export {
  export_faFlyingDisc as faFlyingDisc
};
