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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBird.js
var require_faBird = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBird.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bird";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e469";
    var svgPathData = "M0 192l0-3.6C0 172.7 12.7 160 28.4 160L224 160c17.7 0 32-14.3 32-32l0-32c0-53 43-96 96-96c47.5 0 86.9 34.5 94.6 79.7l57.6 34.6c4.8 2.9 7.8 8.1 7.8 13.7s-2.9 10.8-7.8 13.7L448 175.5l0 16.5c0 93.9-57.7 174.3-139.7 207.6l40.9 77.2c6.2 11.7 1.7 26.2-10 32.4s-26.2 1.7-32.4-10l-45.7-86.3C249 415 236.6 416 224 416c-1 0-2 0-3 0l32.2 60.8c6.2 11.7 1.7 26.2-10 32.4s-26.2 1.7-32.4-10l-48.6-91.9C68.5 380.5 0 294.3 0 192zm376-72a24 24 0 1 0 -48 0 24 24 0 1 0 48 0z";
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
    exports.faBird = exports.definition;
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

// src/_entries/faBird.js
var import_faBird = __toESM(require_faBird());
var export_faBird = import_faBird.definition;
export {
  export_faBird as faBird
};
