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

// src/node_modules/@fortawesome/free-solid-svg-icons/faChildDress.js
var require_faChildDress = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faChildDress.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "child-dress";
    var width = 320;
    var height = 512;
    var aliases = [];
    var unicode = "e59c";
    var svgPathData = "M224 64A64 64 0 1 0 96 64 64 64 0 1 0 224 64zM88 400l0 80c0 17.7 14.3 32 32 32s32-14.3 32-32l0-80 16 0 0 80c0 17.7 14.3 32 32 32s32-14.3 32-32l0-80 17.8 0c10.9 0 18.6-10.7 15.2-21.1l-31.1-93.4 28.7 37.8c10.7 14.1 30.8 16.8 44.8 6.2s16.8-30.7 6.2-44.8L254.6 207c-22.4-29.6-57.5-47-94.6-47s-72.2 17.4-94.6 47L6.5 284.7c-10.7 14.1-7.9 34.2 6.2 44.8s34.2 7.9 44.8-6.2L86.2 285.5 55 378.9C51.6 389.3 59.3 400 70.2 400L88 400z";
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
    exports.faChildDress = exports.definition;
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

// src/_entries/faChildDress.js
var import_faChildDress = __toESM(require_faChildDress());
var export_faChildDress = import_faChildDress.definition;
export {
  export_faChildDress as faChildDress
};
