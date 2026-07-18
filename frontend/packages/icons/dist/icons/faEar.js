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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faEar.js
var require_faEar = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faEar.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "ear";
    var width = 384;
    var height = 512;
    var aliases = [128066];
    var unicode = "f5f0";
    var svgPathData = "M319.9 335.2c-18.2 16.3-30 39.1-35.1 62.9c-5.6 26.3-18.6 51.3-39 71.7c-56.2 56.2-147.4 56.2-203.6 0C14.1 441.7 0 404.9 0 368L0 192C0 86 86 0 192 0S384 86 384 192c0 56.9-24.8 108-64.1 143.2zM128 192c0-35.3 28.7-64 64-64s64 28.7 64 64l0 8c0 13.3 10.7 24 24 24s24-10.7 24-24l0-8c0-61.9-50.1-112-112-112S80 130.1 80 192l0 20.2c0 21.2 12 40.6 31 50.1l14 7c1.8 .9 3 2.8 3 4.9c0 1.6-.7 3.2-2 4.2L88.6 309.6c-10.2 8.5-11.6 23.6-3.1 33.8s23.6 11.6 33.8 3.1l37.4-31.2C169 305.1 176 290.1 176 274.2c0-20.2-11.4-38.8-29.5-47.8l-14-7c-2.7-1.4-4.4-4.1-4.4-7.2l0-20.2z";
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
    exports.faEar = exports.definition;
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

// src/_entries/faEar.js
var import_faEar = __toESM(require_faEar());
var export_faEar = import_faEar.definition;
export {
  export_faEar as faEar
};
