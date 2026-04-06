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

// src/node_modules/@fortawesome/free-solid-svg-icons/fa8.js
var require_fa8 = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/fa8.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "8";
    var width = 320;
    var height = 512;
    var aliases = [];
    var unicode = "38";
    var svgPathData = "M304 160c0-70.7-57.3-128-128-128l-32 0c-70.7 0-128 57.3-128 128 0 34.6 13.7 66 36 89-31.5 23.3-52 60.8-52 103 0 70.7 57.3 128 128 128l64 0c70.7 0 128-57.3 128-128 0-42.2-20.5-79.7-52-103 22.3-23 36-54.4 36-89zM176.1 288l15.9 0c35.3 0 64 28.7 64 64s-28.7 64-64 64l-64 0c-35.3 0-64-28.7-64-64s28.7-64 64-64l48.1 0zm0-64L144 224c-35.3 0-64-28.7-64-64 0-35.3 28.7-64 64-64l32 0c35.3 0 64 28.7 64 64 0 35.3-28.6 64-64 64z";
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
    exports.fa8 = exports.definition;
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

// src/_entries/fa8.js
var import_fa8 = __toESM(require_fa8());
var export_fa8 = import_fa8.definition;
export {
  export_fa8 as fa8
};
