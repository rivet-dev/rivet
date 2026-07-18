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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faArrowDownToSquare.js
var require_faArrowDownToSquare = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faArrowDownToSquare.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "arrow-down-to-square";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e096";
    var svgPathData = "M374.6 246.6c12.5-12.5 12.5-32.8 0-45.3s-32.8-12.5-45.3 0L288 242.7 288 32c0-17.7-14.3-32-32-32s-32 14.3-32 32l0 210.7-41.4-41.4c-12.5-12.5-32.8-12.5-45.3 0s-12.5 32.8 0 45.3l96 96c12.5 12.5 32.8 12.5 45.3 0l96-96zM80 32C35.8 32 0 67.8 0 112L0 432c0 44.2 35.8 80 80 80l352 0c44.2 0 80-35.8 80-80l0-320c0-44.2-35.8-80-80-80l-32 0c-17.7 0-32 14.3-32 32s14.3 32 32 32l32 0c8.8 0 16 7.2 16 16l0 320c0 8.8-7.2 16-16 16L80 448c-8.8 0-16-7.2-16-16l0-320c0-8.8 7.2-16 16-16l32 0c17.7 0 32-14.3 32-32s-14.3-32-32-32L80 32z";
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
    exports.faArrowDownToSquare = exports.definition;
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

// src/_entries/faArrowDownToSquare.js
var import_faArrowDownToSquare = __toESM(require_faArrowDownToSquare());
var export_faArrowDownToSquare = import_faArrowDownToSquare.definition;
export {
  export_faArrowDownToSquare as faArrowDownToSquare
};
