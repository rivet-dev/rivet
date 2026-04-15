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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTirePressureWarning.js
var require_faTirePressureWarning = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTirePressureWarning.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "tire-pressure-warning";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f633";
    var svgPathData = "M384 32c0-17.7 14.3-32 32-32s32 14.3 32 32l0 22.5c0 24.1 3.9 48 11.5 70.8l2.5 7.4c4.8 14.4 11.5 28.6 19.3 44.4c17.5 35.6 33.4 82.6 27.5 136.2c-5.3 48.6-28.1 99-76.8 148.7l0 34c0 8.8-7.2 16-16 16l-32 0c-8.8 0-16-7.2-16-16l0-8-32 0 0 8c0 8.8-7.2 16-16 16l-32 0c-8.8 0-16-7.2-16-16l0-8-32 0 0 8c0 8.8-7.2 16-16 16l-32 0c-8.8 0-16-7.2-16-16l0-8-32 0 0 8c0 8.8-7.2 16-16 16l-32 0c-8.8 0-16-7.2-16-16l0-34C31.3 412.3 8.6 361.9 3.2 313.3c-5.9-53.6 9.9-100.6 27.5-136.2c7.8-15.8 14.5-30 19.3-44.4l2.5-7.4C60.1 102.5 64 78.5 64 54.5L64 32C64 14.3 78.3 0 96 0s32 14.3 32 32l0 22.5c0 31-5 61.7-14.8 91.1l-2.5 7.4c-6.3 19-14.8 36.6-22.6 52.5c-14.7 29.7-25.3 64-21.3 100.9c3.7 34 20.4 74.1 65.7 117.7l246.8 0c45.3-43.6 62-83.7 65.7-117.7c4.1-36.9-6.6-71.1-21.3-100.9c-7.8-15.8-16.3-33.5-22.6-52.5l-2.5-7.4C389 116.2 384 85.4 384 54.5L384 32zM280 120l0 112c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-112c0-13.3 10.7-24 24-24s24 10.7 24 24zM256 288a32 32 0 1 1 0 64 32 32 0 1 1 0-64z";
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
    exports.faTirePressureWarning = exports.definition;
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

// src/_entries/faTirePressureWarning.js
var import_faTirePressureWarning = __toESM(require_faTirePressureWarning());
var export_faTirePressureWarning = import_faTirePressureWarning.definition;
export {
  export_faTirePressureWarning as faTirePressureWarning
};
