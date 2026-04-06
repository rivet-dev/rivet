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

// src/node_modules/@fortawesome/free-solid-svg-icons/faPlaceOfWorship.js
var require_faPlaceOfWorship = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faPlaceOfWorship.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "place-of-worship";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f67f";
    var svgPathData = "M160 79l0 81-44.8 33.6C103.1 202.7 96 216.9 96 232l0 40-60.6 30.3C13.7 313.2 0 335.3 0 359.6L0 448c0 35.3 28.7 64 64 64 74.7 0 85.4 0 320 0l64 0c35.3 0 64-28.7 64-64l0-88.4c0-24.2-13.7-46.4-35.4-57.2L416 272 416 232c0-15.1-7.1-29.3-19.2-38.4L352 160 352 79c0-9.5-4.2-18.5-11.5-24.6L271.4-3.2c-8.9-7.4-21.8-7.4-30.7 0L171.5 54.4C164.2 60.5 160 69.5 160 79zm96 241c35.3 0 64 28.7 64 64l0 80-128 0 0-80c0-35.3 28.7-64 64-64z";
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
    exports.faPlaceOfWorship = exports.definition;
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

// src/_entries/faPlaceOfWorship.js
var import_faPlaceOfWorship = __toESM(require_faPlaceOfWorship());
var export_faPlaceOfWorship = import_faPlaceOfWorship.definition;
export {
  export_faPlaceOfWorship as faPlaceOfWorship
};
