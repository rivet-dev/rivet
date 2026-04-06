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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faEscalator.js
var require_faEscalator = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faEscalator.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "escalator";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e171";
    var svgPathData = "M208 48a48 48 0 1 1 96 0 48 48 0 1 1 -96 0zM405.9 175.8C417.5 165.6 432.5 160 448 160l96 0c35.3 0 64 28.7 64 64s-28.7 64-64 64l-72 0L234.1 496.2C222.5 506.4 207.5 512 192 512l-96 0c-35.3 0-64-28.7-64-64s28.7-64 64-64l72 0L405.9 175.8zM320 192l0 16.4-128 112L192 192c0-35.3 28.7-64 64-64s64 28.7 64 64z";
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
    exports.faEscalator = exports.definition;
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

// src/_entries/faEscalator.js
var import_faEscalator = __toESM(require_faEscalator());
var export_faEscalator = import_faEscalator.definition;
export {
  export_faEscalator as faEscalator
};
