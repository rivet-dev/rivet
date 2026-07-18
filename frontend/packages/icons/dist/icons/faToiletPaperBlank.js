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

// src/node_modules/@fortawesome/free-solid-svg-icons/faToiletPaper.js
var require_faToiletPaper = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faToiletPaper.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "toilet-paper";
    var width = 512;
    var height = 512;
    var aliases = [129531, 63263, "toilet-paper-alt", "toilet-paper-blank"];
    var unicode = "f71e";
    var svgPathData = "M327.1 32c-8.8 11.1-16 23.1-21.9 34.8-21.4 42.7-33.2 98.4-33.2 157.2l0 264c0 30.9-25.1 56-56 56L56 544c-30.9 0-56-25.1-56-56L0 224C0 118 43 32 96 32l231.1 0zM416 416c-53 0-96-86-96-192s43-192 96-192 96 86 96 192-43 192-96 192zm0-128c17.7 0 32-28.7 32-64s-14.3-64-32-64-32 28.7-32 64 14.3 64 32 64z";
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
    exports.faToiletPaper = exports.definition;
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

// src/_entries/faToiletPaperBlank.js
var import_faToiletPaper = __toESM(require_faToiletPaper());
var export_faToiletPaperBlank = import_faToiletPaper.definition;
export {
  export_faToiletPaperBlank as faToiletPaperBlank
};
