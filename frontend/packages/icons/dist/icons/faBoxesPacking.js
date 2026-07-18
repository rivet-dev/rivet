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

// src/node_modules/@fortawesome/free-solid-svg-icons/faBoxesPacking.js
var require_faBoxesPacking = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faBoxesPacking.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "boxes-packing";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e4c7";
    var svgPathData = "M208 0l80 0 0 56c0 13.3 10.7 24 24 24l80 0c13.3 0 24-10.7 24-24l0-56 80 0c26.5 0 48 21.5 48 48l0 416c0 26.5-21.5 48-48 48l-108.8 0c8.2-14.1 12.8-30.5 12.8-48l0-160c10-13.4 16-30 16-48l0-32c0-44.2-35.8-80-80-80l-176 0 0-96c0-26.5 21.5-48 48-48zM32 336l320 0 0 128c0 26.5-21.5 48-48 48L80 512c-26.5 0-48-21.5-48-48l0-128zM48 192l288 0c17.7 0 32 14.3 32 32l0 32c0 17.7-14.3 32-32 32L48 288c-17.7 0-32-14.3-32-32l0-32c0-17.7 14.3-32 32-32z";
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
    exports.faBoxesPacking = exports.definition;
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

// src/_entries/faBoxesPacking.js
var import_faBoxesPacking = __toESM(require_faBoxesPacking());
var export_faBoxesPacking = import_faBoxesPacking.definition;
export {
  export_faBoxesPacking as faBoxesPacking
};
