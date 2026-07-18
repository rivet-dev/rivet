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

// src/node_modules/@fortawesome/free-solid-svg-icons/faWineGlassEmpty.js
var require_faWineGlassEmpty = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faWineGlassEmpty.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "wine-glass-empty";
    var width = 320;
    var height = 512;
    var aliases = ["wine-glass-alt"];
    var unicode = "f5ce";
    var svgPathData = "M64 0C48.7 0 35.6 10.8 32.6 25.7L3.2 173C1.1 183.5 0 194.2 0 205l0 3c0 77.4 55 142 128 156.8l0 115.2-64 0c-17.7 0-32 14.3-32 32s14.3 32 32 32l192 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-64 0 0-115.2C265 350 320 285.4 320 208l0-3c0-10.7-1.1-21.4-3.2-32L287.4 25.7C284.4 10.8 271.3 0 256 0L64 0zm1.9 185.6L90.2 64 229.8 64 254.1 185.6c1.3 6.4 1.9 12.9 1.9 19.4l0 3c0 53-43 96-96 96s-96-43-96-96l0-3c0-6.5 .6-13 1.9-19.4z";
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
    exports.faWineGlassEmpty = exports.definition;
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

// src/_entries/faWineGlassAlt.js
var import_faWineGlassEmpty = __toESM(require_faWineGlassEmpty());
var export_faWineGlassAlt = import_faWineGlassEmpty.definition;
export {
  export_faWineGlassAlt as faWineGlassAlt
};
