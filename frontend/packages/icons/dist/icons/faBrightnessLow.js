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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBrightnessLow.js
var require_faBrightnessLow = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBrightnessLow.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "brightness-low";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e0ca";
    var svgPathData = "M224 80a32 32 0 1 1 64 0 32 32 0 1 1 -64 0zM160 256a96 96 0 1 1 192 0 96 96 0 1 1 -192 0zm96 144a32 32 0 1 1 0 64 32 32 0 1 1 0-64zM432 224a32 32 0 1 1 0 64 32 32 0 1 1 0-64zM112 256a32 32 0 1 1 -64 0 32 32 0 1 1 64 0zM96 128a32 32 0 1 1 64 0 32 32 0 1 1 -64 0zM352 384a32 32 0 1 1 64 0 32 32 0 1 1 -64 0zm0-256a32 32 0 1 1 64 0 32 32 0 1 1 -64 0zM160 384a32 32 0 1 1 -64 0 32 32 0 1 1 64 0z";
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
    exports.faBrightnessLow = exports.definition;
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

// src/_entries/faBrightnessLow.js
var import_faBrightnessLow = __toESM(require_faBrightnessLow());
var export_faBrightnessLow = import_faBrightnessLow.definition;
export {
  export_faBrightnessLow as faBrightnessLow
};
