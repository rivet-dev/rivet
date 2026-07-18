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

// src/node_modules/@fortawesome/free-solid-svg-icons/fa9.js
var require_fa9 = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/fa9.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "9";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "39";
    var svgPathData = "M208 320c29.6 0 57.1-8.9 80-24.2l0 32.2c0 48.6-39.4 88-88 88L96 416c-17.7 0-32 14.3-32 32s14.3 32 32 32l104 0c83.9 0 152-68.1 152-152l0-152.1C351.9 96.4 287.5 32 208 32l-48 0C80.5 32 16 96.5 16 176S80.5 320 160 320l48 0zm80-144c0 44.2-35.8 80-80 80l-48 0c-44.2 0-80-35.8-80-80s35.8-80 80-80l48 0c44.2 0 80 35.8 80 80z";
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
    exports.fa9 = exports.definition;
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

// src/_entries/fa9.js
var import_fa9 = __toESM(require_fa9());
var export_fa9 = import_fa9.definition;
export {
  export_fa9 as fa9
};
