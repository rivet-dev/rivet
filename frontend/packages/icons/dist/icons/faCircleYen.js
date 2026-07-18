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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCircleYen.js
var require_faCircleYen = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCircleYen.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "circle-yen";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e5d0";
    var svgPathData = "M256 512A256 256 0 1 0 256 0a256 256 0 1 0 0 512zM159.9 150.2c9.8-8.9 25-8.2 33.9 1.6L256 220.3l62.2-68.5c8.9-9.8 24.1-10.5 33.9-1.6s10.5 24.1 1.6 33.9L288.4 256l31.6 0c8.8 0 16 7.2 16 16s-7.2 16-16 16l-40 0 0 32 40 0c8.8 0 16 7.2 16 16s-7.2 16-16 16l-40 0 0 25c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-25-40 0c-8.8 0-16-7.2-16-16s7.2-16 16-16l40 0 0-32-40 0c-8.8 0-16-7.2-16-16s7.2-16 16-16l31.6 0-65.3-71.9c-8.9-9.8-8.2-25 1.6-33.9z";
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
    exports.faCircleYen = exports.definition;
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

// src/_entries/faCircleYen.js
var import_faCircleYen = __toESM(require_faCircleYen());
var export_faCircleYen = import_faCircleYen.definition;
export {
  export_faCircleYen as faCircleYen
};
