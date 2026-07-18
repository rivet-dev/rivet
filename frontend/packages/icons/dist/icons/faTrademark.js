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

// src/node_modules/@fortawesome/free-solid-svg-icons/faTrademark.js
var require_faTrademark = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faTrademark.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "trademark";
    var width = 640;
    var height = 512;
    var aliases = [8482];
    var unicode = "f25c";
    var svgPathData = "M346.6 110.2c-7.8-11.7-22.4-17-35.9-12.9S288 113.9 288 128l0 256c0 17.7 14.3 32 32 32s32-14.3 32-32l0-150.3 69.4 104.1c5.9 8.9 15.9 14.2 26.6 14.2s20.7-5.3 26.6-14.2L544 233.7 544 384c0 17.7 14.3 32 32 32s32-14.3 32-32l0-256c0-14.1-9.2-26.5-22.7-30.6s-28.1 1.1-35.9 12.9L448 262.3 346.6 110.2zM32 96C14.3 96 0 110.3 0 128s14.3 32 32 32l64 0 0 224c0 17.7 14.3 32 32 32s32-14.3 32-32l0-224 64 0c17.7 0 32-14.3 32-32s-14.3-32-32-32L32 96z";
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
    exports.faTrademark = exports.definition;
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

// src/_entries/faTrademark.js
var import_faTrademark = __toESM(require_faTrademark());
var export_faTrademark = import_faTrademark.definition;
export {
  export_faTrademark as faTrademark
};
