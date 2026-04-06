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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPipeValve.js
var require_faPipeValve = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPipeValve.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "pipe-valve";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e439";
    var svgPathData = "M288 64c0-17.7-14.3-32-32-32s-32 14.3-32 32l0 12L128 64c-17.7 0-32 14.3-32 32s14.3 32 32 32l128-16 128 16c17.7 0 32-14.3 32-32s-14.3-32-32-32L288 76l0-12zm0 84.2l-32-4-32 4 0 43.8L96 192l0 256 320 0 0-256-128 0 0-43.8zM32 160c-17.7 0-32 14.3-32 32L0 448c0 17.7 14.3 32 32 32s32-14.3 32-32l0-256c0-17.7-14.3-32-32-32zm480 32c0-17.7-14.3-32-32-32s-32 14.3-32 32l0 256c0 17.7 14.3 32 32 32s32-14.3 32-32l0-256z";
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
    exports.faPipeValve = exports.definition;
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

// src/_entries/faPipeValve.js
var import_faPipeValve = __toESM(require_faPipeValve());
var export_faPipeValve = import_faPipeValve.definition;
export {
  export_faPipeValve as faPipeValve
};
