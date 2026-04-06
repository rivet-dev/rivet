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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSquareX.js
var require_faSquareX = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSquareX.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "square-x";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e286";
    var svgPathData = "M64 32C28.7 32 0 60.7 0 96L0 416c0 35.3 28.7 64 64 64l320 0c35.3 0 64-28.7 64-64l0-320c0-35.3-28.7-64-64-64L64 32zm90.3 104.5L224 218.8l69.7-82.3c8.6-10.1 23.7-11.4 33.8-2.8s11.4 23.7 2.8 33.8L255.4 256l74.9 88.5c8.6 10.1 7.3 25.3-2.8 33.8s-25.3 7.3-33.8-2.8L224 293.2l-69.7 82.3c-8.6 10.1-23.7 11.4-33.8 2.8s-11.4-23.7-2.8-33.8L192.6 256l-74.9-88.5c-8.6-10.1-7.3-25.3 2.8-33.8s25.3-7.3 33.8 2.8z";
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
    exports.faSquareX = exports.definition;
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

// src/_entries/faSquareX.js
var import_faSquareX = __toESM(require_faSquareX());
var export_faSquareX = import_faSquareX.definition;
export {
  export_faSquareX as faSquareX
};
