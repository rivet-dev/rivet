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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTurtle.js
var require_faTurtle = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTurtle.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "turtle";
    var width = 576;
    var height = 512;
    var aliases = [128034];
    var unicode = "f726";
    var svgPathData = "M384 249.2c0 21.4-17.4 38.8-38.8 38.8L70.8 288C49.4 288 32 270.6 32 249.2L32 208c0-97.2 78.8-176 176-176s176 78.8 176 176l0 41.2zM480 288c-2 0-4-.1-5.9-.3c-16.3 53.3-64.3 92.7-122.1 96l0 56.2c0 22.1-17.9 40-40 40l-48 0c-22.1 0-40-17.9-40-40l0-56-32 0 0 56c0 22.1-17.9 40-40 40l-48 0c-22.1 0-40-17.9-40-40l0-56-32 0c-17.7 0-32-14.3-32-32s14.3-32 32-32l72 0 48 0 112 0 48 0 32 0c39.8 0 72-32.2 72-72l0-24 0-32 0-16c0-35.3 28.7-64 64-64c53 0 96 43 96 96l0 16c0 35.3-28.7 64-64 64l-32 0zm16-64a16 16 0 1 0 0-32 16 16 0 1 0 0 32z";
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
    exports.faTurtle = exports.definition;
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

// src/_entries/faTurtle.js
var import_faTurtle = __toESM(require_faTurtle());
var export_faTurtle = import_faTurtle.definition;
export {
  export_faTurtle as faTurtle
};
