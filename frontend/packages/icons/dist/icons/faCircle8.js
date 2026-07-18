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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCircle8.js
var require_faCircle8 = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCircle8.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "circle-8";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e0f5";
    var svgPathData = "M0 256a256 256 0 1 1 512 0A256 256 0 1 1 0 256zM264 128l-16 0c-39.8 0-72 32.2-72 72c0 15.7 5 30.1 13.5 42c-18 14.7-29.5 37-29.5 62c0 44.2 35.8 80 80 80l32 0c44.2 0 80-35.8 80-80c0-25-11.5-47.4-29.5-62c8.5-11.8 13.5-26.3 13.5-42c0-39.8-32.2-72-72-72zm-16 96c-13.2 0-24-10.8-24-24s10.7-24 24-24l16 0c13.3 0 24 10.7 24 24s-10.7 24-24 24c0 0 0 0 0 0l-16 0s0 0 0 0zm16 48l8 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-32 0c-17.7 0-32-14.3-32-32s14.3-32 32-32l8 0 16 0z";
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
    exports.faCircle8 = exports.definition;
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

// src/_entries/faCircle8.js
var import_faCircle8 = __toESM(require_faCircle8());
var export_faCircle8 = import_faCircle8.definition;
export {
  export_faCircle8 as faCircle8
};
