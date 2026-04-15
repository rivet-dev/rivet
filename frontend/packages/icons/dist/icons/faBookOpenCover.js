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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBookOpenCover.js
var require_faBookOpenCover = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBookOpenCover.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "book-open-cover";
    var width = 640;
    var height = 512;
    var aliases = ["book-open-alt"];
    var unicode = "e0c0";
    var svgPathData = "M304 32l0 376L96 368 96 34.7C96 14.9 113.8-.2 133.3 3.1L304 32zM89.7 405.1L320 451.2l230.3-46.1c15-3 25.7-16.1 25.7-31.4l0-345 25.7-5.1C621.5 19.7 640 34.8 640 55l0 366.7c0 15.3-10.8 28.4-25.7 31.4L320 512 25.7 453.1C10.8 450.2 0 437 0 421.8L0 55C0 34.8 18.5 19.7 38.3 23.7L64 28.8l0 345c0 15.3 10.8 28.4 25.7 31.4zM336 408l0-376L506.7 3.1C526.2-.2 544 14.9 544 34.7L544 368 336 408z";
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
    exports.faBookOpenCover = exports.definition;
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

// src/_entries/faBookOpenCover.js
var import_faBookOpenCover = __toESM(require_faBookOpenCover());
var export_faBookOpenCover = import_faBookOpenCover.definition;
export {
  export_faBookOpenCover as faBookOpenCover
};
