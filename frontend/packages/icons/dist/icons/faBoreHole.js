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

// src/node_modules/@fortawesome/free-solid-svg-icons/faBoreHole.js
var require_faBoreHole = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faBoreHole.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bore-hole";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e4c3";
    var svgPathData = "M256 0c-17.7 0-32 14.3-32 32l0 232.6c-19.1 11.1-32 31.7-32 55.4 0 35.3 28.7 64 64 64s64-28.7 64-64c0-23.7-12.9-44.4-32-55.4L288 32c0-17.7-14.3-32-32-32zM0 384l0 64c0 35.3 28.7 64 64 64l384 0c35.3 0 64-28.7 64-64l0-272c0-26.5-21.5-48-48-48l-48 0c-26.5 0-48 21.5-48 48l0 144c0 61.9-50.1 112-112 112S144 381.9 144 320l0-144c0-26.5-21.5-48-48-48l-48 0c-26.5 0-48 21.5-48 48L0 384z";
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
    exports.faBoreHole = exports.definition;
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

// src/_entries/faBoreHole.js
var import_faBoreHole = __toESM(require_faBoreHole());
var export_faBoreHole = import_faBoreHole.definition;
export {
  export_faBoreHole as faBoreHole
};
