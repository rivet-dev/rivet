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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faDagger.js
var require_faDagger = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faDagger.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "dagger";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "f6cb";
    var svgPathData = "M224 32c0-17.7-14.3-32-32-32s-32 14.3-32 32l0 96L48 128c-26.5 0-48 21.5-48 48s21.5 48 48 48c20.9 0 38.7-13.4 45.3-32l98.7 0 98.7 0c6.6 18.6 24.4 32 45.3 32c26.5 0 48-21.5 48-48s-21.5-48-48-48l-112 0 0-96zM128 224l0 24 0 166.8c0 10.5 2.9 20.8 8.5 29.7l35.1 56.2c4.4 7 12.1 11.3 20.4 11.3s16-4.3 20.4-11.3l35.1-56.2c5.6-8.9 8.5-19.2 8.5-29.7L256 248l0-24-24 0-80 0-24 0z";
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
    exports.faDagger = exports.definition;
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

// src/_entries/faDagger.js
var import_faDagger = __toESM(require_faDagger());
var export_faDagger = import_faDagger.definition;
export {
  export_faDagger as faDagger
};
