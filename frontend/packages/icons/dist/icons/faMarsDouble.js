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

// src/node_modules/@fortawesome/free-solid-svg-icons/faMarsDouble.js
var require_faMarsDouble = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faMarsDouble.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "mars-double";
    var width = 640;
    var height = 512;
    var aliases = [9891];
    var unicode = "f227";
    var svgPathData = "M288 32c0-17.7 14.3-32 32-32L448 0c17.7 0 32 14.3 32 32l0 128c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-50.7-95 95c19.5 28.4 31 62.7 31 99.8 0 97.2-78.8 176-176 176S0 401.2 0 304 78.8 128 176 128c37 0 71.4 11.4 99.8 31l95-95-50.7 0c-17.7 0-32-14.3-32-32zM176 416a112 112 0 1 0 0-224 112 112 0 1 0 0 224zm160 64c-6.8 0-13.4-.4-20-1.1 23.6-18.9 43.3-42.5 57.6-69.4 43.3-15.5 74.4-56.9 74.4-105.5 0-25.2-8.3-48.5-22.4-67.2 7.1 2.1 14.6 3.2 22.4 3.2 17 0 32.7-5.3 45.7-14.3 11.7 23.6 18.3 50.2 18.3 78.3 0 97.2-78.8 176-176 176zM528 157.3L528 32c0-11.4-2.4-22.2-6.7-32L608 0c17.7 0 32 14.3 32 32l0 128c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-50.7-48 48z";
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
    exports.faMarsDouble = exports.definition;
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

// src/_entries/faMarsDouble.js
var import_faMarsDouble = __toESM(require_faMarsDouble());
var export_faMarsDouble = import_faMarsDouble.definition;
export {
  export_faMarsDouble as faMarsDouble
};
