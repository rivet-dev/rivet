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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faComputerSpeaker.js
var require_faComputerSpeaker = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faComputerSpeaker.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "computer-speaker";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f8b2";
    var svgPathData = "M288 432l0-16-160 0c-17.7 0-32 14.3-32 32s14.3 32 32 32l176 0c-10-13.4-16-30-16-48zM64 32C28.7 32 0 60.7 0 96L0 320c0 35.3 28.7 64 64 64l224 0 0-64L64 320 64 96l224 0 0-16c0-18 6-34.6 16-48L64 32zm304 0c-26.5 0-48 21.5-48 48l0 352c0 26.5 21.5 48 48 48l224 0c26.5 0 48-21.5 48-48l0-352c0-26.5-21.5-48-48-48L368 32zM544 320a64 64 0 1 0 -128 0 64 64 0 1 0 128 0zm-160 0a96 96 0 1 1 192 0 96 96 0 1 1 -192 0zm64-192a32 32 0 1 1 64 0 32 32 0 1 1 -64 0z";
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
    exports.faComputerSpeaker = exports.definition;
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

// src/_entries/faComputerSpeaker.js
var import_faComputerSpeaker = __toESM(require_faComputerSpeaker());
var export_faComputerSpeaker = import_faComputerSpeaker.definition;
export {
  export_faComputerSpeaker as faComputerSpeaker
};
