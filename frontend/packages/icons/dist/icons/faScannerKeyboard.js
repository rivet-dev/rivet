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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faScannerKeyboard.js
var require_faScannerKeyboard = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faScannerKeyboard.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "scanner-keyboard";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f489";
    var svgPathData = "M152 0l16 0c13.3 0 24 10.7 24 24l0 40-64 0 0-40c0-13.3 10.7-24 24-24zM312 0l16 0c13.3 0 24 10.7 24 24l0 40-64 0 0-40c0-13.3 10.7-24 24-24zM224 16c0-8.8 7.2-16 16-16s16 7.2 16 16l0 48-32 0 0-48zM400 0c8.8 0 16 7.2 16 16l0 256c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-256c0-8.8 7.2-16 16-16zm48 24c0-13.3 10.7-24 24-24l16 0c13.3 0 24 10.7 24 24l0 240c0 13.3-10.7 24-24 24l-16 0c-13.3 0-24-10.7-24-24l0-240zM0 160c0-35.3 28.7-64 64-64l224 0c35.3 0 64 28.7 64 64l0 288c0 35.3-28.7 64-64 64L64 512c-35.3 0-64-28.7-64-64L0 160zm64 24l0 16c0 13.3 10.7 24 24 24l176 0c13.3 0 24-10.7 24-24l0-16c0-13.3-10.7-24-24-24L88 160c-13.3 0-24 10.7-24 24zm0 120l0 32c0 8.8 7.2 16 16 16l64 0c8.8 0 16-7.2 16-16l0-32c0-8.8-7.2-16-16-16l-64 0c-8.8 0-16 7.2-16 16zm16 80c-8.8 0-16 7.2-16 16l0 32c0 8.8 7.2 16 16 16l64 0c8.8 0 16-7.2 16-16l0-32c0-8.8-7.2-16-16-16l-64 0zm112-80l0 32c0 8.8 7.2 16 16 16l64 0c8.8 0 16-7.2 16-16l0-32c0-8.8-7.2-16-16-16l-64 0c-8.8 0-16 7.2-16 16zm16 80c-8.8 0-16 7.2-16 16l0 32c0 8.8 7.2 16 16 16l64 0c8.8 0 16-7.2 16-16l0-32c0-8.8-7.2-16-16-16l-64 0z";
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
    exports.faScannerKeyboard = exports.definition;
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

// src/_entries/faScannerKeyboard.js
var import_faScannerKeyboard = __toESM(require_faScannerKeyboard());
var export_faScannerKeyboard = import_faScannerKeyboard.definition;
export {
  export_faScannerKeyboard as faScannerKeyboard
};
