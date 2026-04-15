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

// src/node_modules/@fortawesome/free-solid-svg-icons/faDisease.js
var require_faDisease = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faDisease.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "disease";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f7fa";
    var svgPathData = "M236.5 29.4L224 48c-20 30-53.6 48-89.7 48L68 96c-37.5 0-67.9 30.4-67.9 67.9 0 18 7.2 35.2 19.9 47.9l27 27c11 11 17.2 25.9 17.2 41.5 0 15.8-6.4 30.9-17.7 42L33.4 335.1c-11.1 10.8-17.3 25.7-17.3 41.2 0 36.8 34.1 64.2 70.1 56.2l62.3-13.8c7.7-1.7 15.7-2.6 23.6-2.6 32.8 0 64.2 14.6 85.2 39.8l30.5 36.6c10.4 12.4 25.7 19.6 41.9 19.6 30.1 0 54.5-24.4 54.5-54.5l0-51.2c0-41.4 25.4-78.5 64-93.5l22.2-8.6c25.2-9.8 41.8-34.1 41.8-61.1 0-26.4-15.9-50.3-40.3-60.5L429 164.8c-33.2-13.9-57.6-43-65.5-78.1l-7.9-35.2c-6.8-30.1-33.5-51.5-64.3-51.5-22 0-42.6 11-54.8 29.4zM160 192a32 32 0 1 1 0 64 32 32 0 1 1 0-64zm96 0a32 32 0 1 1 64 0 32 32 0 1 1 -64 0zm32 96a32 32 0 1 1 0 64 32 32 0 1 1 0-64z";
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
    exports.faDisease = exports.definition;
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

// src/_entries/faDisease.js
var import_faDisease = __toESM(require_faDisease());
var export_faDisease = import_faDisease.definition;
export {
  export_faDisease as faDisease
};
