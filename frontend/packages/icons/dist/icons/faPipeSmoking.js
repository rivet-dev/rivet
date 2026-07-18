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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPipeSmoking.js
var require_faPipeSmoking = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPipeSmoking.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "pipe-smoking";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e3c4";
    var svgPathData = "M575.2 64c-32.1 0-62.1 16-79.9 42.7l-61.8 92.7c-3.9-4.8-8.6-9.1-14-12.7l-24-16c-4.7-3.2-9.7-5.6-14.9-7.4L442 71.2C471.7 26.7 521.7 0 575.2 0L608 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-32.8 0zM333.4 206.2c9.8-14.7 29.7-18.7 44.4-8.9l24 16c14.7 9.8 18.7 29.7 8.9 44.4L287.8 442.1C258.6 485.8 209.6 512 157.1 512C70.3 512 0 441.7 0 354.9L0 272c0-26.5 21.5-48 48-48l160 0c26.5 0 48 21.5 48 48l0 50.3 77.4-116.1z";
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
    exports.faPipeSmoking = exports.definition;
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

// src/_entries/faPipeSmoking.js
var import_faPipeSmoking = __toESM(require_faPipeSmoking());
var export_faPipeSmoking = import_faPipeSmoking.definition;
export {
  export_faPipeSmoking as faPipeSmoking
};
