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

// src/node_modules/@fortawesome/free-solid-svg-icons/faNonBinary.js
var require_faNonBinary = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faNonBinary.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "non-binary";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "e807";
    var svgPathData = "M192 544c-97.2 0-176-78.8-176-176 0-86.3 62.1-158 144-173l0-47.2-49.7 24.8-3 1.3c-15.2 5.7-32.5-.8-39.9-15.7-7.4-14.8-2.2-32.6 11.5-41.3l2.8-1.6 38.8-19.4-38.8-19.4c-15.8-7.9-22.2-27.1-14.3-42.9 7.4-14.8 24.8-21.4 40-15.6l3 1.3 49.7 24.8 0-44.2c0-17.7 14.3-32 32-32s32 14.3 32 32l0 44.2 49.7-24.8 3-1.3c15.2-5.8 32.5 .8 39.9 15.6s2.2 32.7-11.5 41.3l-2.8 1.6-38.7 19.4 38.7 19.3c15.8 7.9 22.2 27.1 14.3 42.9-7.4 14.8-24.7 21.4-39.9 15.6l-3-1.3-49.7-24.8 0 47.2c81.9 15.1 144 86.8 144 173 0 97.2-78.8 176-176 176zm0-64a112 112 0 1 0 0-224 112 112 0 1 0 0 224z";
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
    exports.faNonBinary = exports.definition;
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

// src/_entries/faNonBinary.js
var import_faNonBinary = __toESM(require_faNonBinary());
var export_faNonBinary = import_faNonBinary.definition;
export {
  export_faNonBinary as faNonBinary
};
