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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceClouds.js
var require_faFaceClouds = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceClouds.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-clouds";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e47d";
    var svgPathData = "M128 0C104.3 0 83.6 12.9 72.6 32L64 32C28.7 32 0 60.7 0 96s28.7 64 64 64l160 0c35.3 0 64-28.7 64-64s-28.7-64-64-64c-13.2 0-25.4 4-35.6 10.8C179.7 17.9 155.9 0 128 0zM72.1 192c-5.3 20.5-8.1 41.9-8.1 64c0 114.6 75.4 211.7 179.3 244.3c-12.1-17-19.3-37.8-19.3-60.3c0-50.6 36.2-92.8 84.1-102.1C321.2 290.7 364.6 256 416 256c29.1 0 55.6 11.1 75.5 29.2c8.9-3.4 18.5-5.2 28.5-5.2c20 0 38.3 7.3 52.3 19.5c2.4-14.1 3.7-28.7 3.7-43.5C576 114.6 461.4 0 320 0c-20.6 0-40.7 2.4-59.9 7C295.2 21.3 320 55.8 320 96c0 39.6-24 73.6-58.2 88.3c6.5 5.9 10.5 14.3 10.5 23.7c0 17.7-14.3 32-32 32s-32-14.3-32-32c0-5.8 1.6-11.3 4.3-16L72.1 192zm328.3-16a32 32 0 1 1 0 64 32 32 0 1 1 0-64zM336 368l-8 0c-39.8 0-72 32.2-72 72s32.2 72 72 72l240 0c39.8 0 72-32.2 72-72s-32.2-72-72-72l-.7 0c.4-2.6 .7-5.3 .7-8c0-26.5-21.5-48-48-48c-13.9 0-26.4 5.9-35.1 15.3C470.9 303.8 445.3 288 416 288c-44.2 0-80 35.8-80 80z";
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
    exports.faFaceClouds = exports.definition;
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

// src/_entries/faFaceClouds.js
var import_faFaceClouds = __toESM(require_faFaceClouds());
var export_faFaceClouds = import_faFaceClouds.definition;
export {
  export_faFaceClouds as faFaceClouds
};
