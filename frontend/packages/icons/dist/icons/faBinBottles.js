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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBinBottles.js
var require_faBinBottles = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBinBottles.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bin-bottles";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e5f5";
    var svgPathData = "M152 0l80 0c13.3 0 24 10.7 24 24l0 8c8.8 0 16 7.2 16 16s-7.2 16-16 16L128 64c-8.8 0-16-7.2-16-16s7.2-16 16-16l0-8c0-13.3 10.7-24 24-24zM32 192c0-53 43-96 96-96l128 0c24.9 0 47.6 9.5 64.6 25c1.1 1 2.2 2 3.2 3.1C341.3 141.5 352 165.5 352 192L32 192zm352 0c0-40.7-19-76.9-48.6-100.4c5.2-5.6 11.5-10.3 18.6-13.6l13.2-6.2c5.3-2.5 10.8-4.4 16.4-5.7l0-42.1c0-13.3 10.7-24 24-24l80 0c13.3 0 24 10.7 24 24l0 42.1c5.6 1.3 11.1 3.2 16.4 5.7L541.2 78c21 9.9 34.4 31 34.4 54.2c0 12.3-3.7 23.8-10.1 33.3c5 7.8 8.4 16.9 9.6 26.5L384 192zM0 256c0-17.7 14.3-32 32-32l576 0c17.7 0 32 14.3 32 32s-14.3 32-32 32L583.8 457c-4.5 31.5-31.5 55-63.4 55l-401 0c-31.8 0-58.9-23.4-63.4-55L32 288c-17.7 0-32-14.3-32-32z";
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
    exports.faBinBottles = exports.definition;
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

// src/_entries/faBinBottles.js
var import_faBinBottles = __toESM(require_faBinBottles());
var export_faBinBottles = import_faBinBottles.definition;
export {
  export_faBinBottles as faBinBottles
};
