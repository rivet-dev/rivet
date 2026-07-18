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

// src/node_modules/@fortawesome/free-solid-svg-icons/faEraser.js
var require_faEraser = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faEraser.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "eraser";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f12d";
    var svgPathData = "M178.5 416l123 0 65.3-65.3-173.5-173.5-126.7 126.7 112 112zM224 480l-45.5 0c-17 0-33.3-6.7-45.3-18.7L17 345C6.1 334.1 0 319.4 0 304s6.1-30.1 17-41L263 17C273.9 6.1 288.6 0 304 0s30.1 6.1 41 17L527 199c10.9 10.9 17 25.6 17 41s-6.1 30.1-17 41l-135 135 120 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-288 0z";
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
    exports.faEraser = exports.definition;
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

// src/_entries/faEraser.js
var import_faEraser = __toESM(require_faEraser());
var export_faEraser = import_faEraser.definition;
export {
  export_faEraser as faEraser
};
