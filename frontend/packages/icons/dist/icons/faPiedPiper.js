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

// src/node_modules/@fortawesome/free-brands-svg-icons/faPiedPiper.js
var require_faPiedPiper = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faPiedPiper.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "pied-piper";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f2ae";
    var svgPathData = "M440.2 23.2c-26.7 6.8-68.1 28.5-114.6 67.5-30.9-17.5-65.8-26.7-101.4-26.7-114.9 0-208 93.1-208 208s93.1 208 208 208 208-93.1 208-208c.1-54.1-21-106.1-58.7-144.8-6.6 8.5-12.3 17.7-17 27.4 28.9 32.3 44.8 74.1 44.9 117.4 0 97.7-79.4 177.1-177.1 177.1-30.8 0-61-8.1-87.6-23.4 82.9-107.3 150.8-37.8 184.3-226.6 5.8-32.6 28-94.3 126.2-160.2 8.1-5.4 2.4-18.1-7-15.7zM109.3 406.4C89.8 389.8 74.2 369.2 63.5 346s-16.3-48.5-16.3-74c0-97.7 79.4-177.1 177.1-177.1 26.6 0 52.8 6.1 76.6 17.8-66 62.1-126.9 152.9-191.6 293.8z";
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
    exports.faPiedPiper = exports.definition;
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

// src/_entries/faPiedPiper.js
var import_faPiedPiper = __toESM(require_faPiedPiper());
var export_faPiedPiper = import_faPiedPiper.definition;
export {
  export_faPiedPiper as faPiedPiper
};
