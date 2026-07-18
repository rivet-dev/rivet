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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faScannerTouchscreen.js
var require_faScannerTouchscreen = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faScannerTouchscreen.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "scanner-touchscreen";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f48a";
    var svgPathData = "M152 0l16 0c13.3 0 24 10.7 24 24l0 40-64 0 0-40c0-13.3 10.7-24 24-24zM0 160c0-35.3 28.7-64 64-64l224 0c35.3 0 64 28.7 64 64l0 288c0 35.3-28.7 64-64 64L64 512c-35.3 0-64-28.7-64-64L0 160zm64 16l0 256c0 8.8 7.2 16 16 16l192 0c8.8 0 16-7.2 16-16l0-256c0-8.8-7.2-16-16-16L80 160c-8.8 0-16 7.2-16 16zM288 24c0-13.3 10.7-24 24-24l16 0c13.3 0 24 10.7 24 24l0 40-64 0 0-40zM240 0c8.8 0 16 7.2 16 16l0 48-32 0 0-48c0-8.8 7.2-16 16-16zM384 16c0-8.8 7.2-16 16-16s16 7.2 16 16l0 256c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-256zM472 0l16 0c13.3 0 24 10.7 24 24l0 240c0 13.3-10.7 24-24 24l-16 0c-13.3 0-24-10.7-24-24l0-240c0-13.3 10.7-24 24-24z";
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
    exports.faScannerTouchscreen = exports.definition;
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

// src/_entries/faScannerTouchscreen.js
var import_faScannerTouchscreen = __toESM(require_faScannerTouchscreen());
var export_faScannerTouchscreen = import_faScannerTouchscreen.definition;
export {
  export_faScannerTouchscreen as faScannerTouchscreen
};
