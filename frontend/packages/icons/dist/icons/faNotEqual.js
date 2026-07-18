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

// src/node_modules/@fortawesome/free-solid-svg-icons/faNotEqual.js
var require_faNotEqual = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faNotEqual.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "not-equal";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f53e";
    var svgPathData = "M378.6 81.8c9.8-14.7 5.8-34.6-8.9-44.4s-34.6-5.8-44.4 8.9L270.9 128 32 128c-17.7 0-32 14.3-32 32s14.3 32 32 32l196.2 0-85.3 128-110.9 0c-17.7 0-32 14.3-32 32s14.3 32 32 32l68.2 0-30.8 46.2c-9.8 14.7-5.8 34.6 8.9 44.4s34.6 5.8 44.4-8.9L177.1 384 416 384c17.7 0 32-14.3 32-32s-14.3-32-32-32l-196.2 0 85.3-128 110.9 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-68.2 0 30.8-46.2z";
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
    exports.faNotEqual = exports.definition;
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

// src/_entries/faNotEqual.js
var import_faNotEqual = __toESM(require_faNotEqual());
var export_faNotEqual = import_faNotEqual.definition;
export {
  export_faNotEqual as faNotEqual
};
