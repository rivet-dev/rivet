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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faAirConditioner.js
var require_faAirConditioner = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faAirConditioner.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "air-conditioner";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f8f4";
    var svgPathData = "M64 0C28.7 0 0 28.7 0 64l0 96c0 35.3 28.7 64 64 64l448 0c35.3 0 64-28.7 64-64l0-96c0-35.3-28.7-64-64-64L64 0zM80 128l416 0c8.8 0 16 7.2 16 16s-7.2 16-16 16L80 160c-8.8 0-16-7.2-16-16s7.2-16 16-16zM256 256l-64 0 0 160c0 17.7-14.3 32-32 32s-32-14.3-32-32c0-11.8 6.4-22.2 16-27.7c15.3-8.9 20.5-28.4 11.7-43.7s-28.4-20.5-43.7-11.7C83.4 349.4 64 380.4 64 416c0 53 43 96 96 96s96-43 96-96l0-160zm64 128c0 53 43 96 96 96s96-43 96-96c0-35.6-19.4-66.6-48-83.1c-15.3-8.8-34.9-3.6-43.7 11.7s-3.6 34.9 11.7 43.7c9.6 5.6 16 15.9 16 27.7c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-128-64 0 0 128z";
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
    exports.faAirConditioner = exports.definition;
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

// src/_entries/faAirConditioner.js
var import_faAirConditioner = __toESM(require_faAirConditioner());
var export_faAirConditioner = import_faAirConditioner.definition;
export {
  export_faAirConditioner as faAirConditioner
};
