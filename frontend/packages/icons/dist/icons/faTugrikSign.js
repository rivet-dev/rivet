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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTugrikSign.js
var require_faTugrikSign = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTugrikSign.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "tugrik-sign";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "e2ba";
    var svgPathData = "M32 32C14.3 32 0 46.3 0 64S14.3 96 32 96l128 0 0 103L56.2 225c-17.1 4.3-27.6 21.7-23.3 38.8s21.7 27.6 38.8 23.3L160 265l0 30L56.2 321c-17.1 4.3-27.6 21.7-23.3 38.8s21.7 27.6 38.8 23.3L160 361l0 87c0 17.7 14.3 32 32 32s32-14.3 32-32l0-103L327.8 319c17.1-4.3 27.6-21.7 23.3-38.8s-21.7-27.6-38.8-23.3L224 279l0-30L327.8 223c17.1-4.3 27.6-21.7 23.3-38.8s-21.7-27.6-38.8-23.3L224 183l0-87 128 0c17.7 0 32-14.3 32-32s-14.3-32-32-32L192 32 32 32z";
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
    exports.faTugrikSign = exports.definition;
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

// src/_entries/faTugrikSign.js
var import_faTugrikSign = __toESM(require_faTugrikSign());
var export_faTugrikSign = import_faTugrikSign.definition;
export {
  export_faTugrikSign as faTugrikSign
};
