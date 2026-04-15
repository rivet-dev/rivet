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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faKiwiFruit.js
var require_faKiwiFruit = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faKiwiFruit.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "kiwi-fruit";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e30c";
    var svgPathData = "M448 176c0 79.5-100.3 144-224 144S0 255.5 0 176S100.3 32 224 32s224 64.5 224 144zM224 64c-8.8 0-16 7.2-16 16l0 24c0 8.8 7.2 16 16 16s16-7.2 16-16l0-24c0-8.8-7.2-16-16-16zM64 176c0 8.8 7.2 16 16 16l32 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-32 0c-8.8 0-16 7.2-16 16zm256 0c0 8.8 7.2 16 16 16l32 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-32 0c-8.8 0-16 7.2-16 16zm-96 56c-8.8 0-16 7.2-16 16l0 24c0 8.8 7.2 16 16 16s16-7.2 16-16l0-24c0-8.8-7.2-16-16-16zM139.3 92.7c-6.2-6.2-16.4-6.2-22.6 0s-6.2 16.4 0 22.6l16 16c6.2 6.2 16.4 6.2 22.6 0s6.2-16.4 0-22.6l-16-16zm192 0c-6.2-6.2-16.4-6.2-22.6 0l-16 16c-6.2 6.2-6.2 16.4 0 22.6s16.4 6.2 22.6 0l16-16c6.2-6.2 6.2-16.4 0-22.6zm-176 150.6c6.2-6.2 6.2-16.4 0-22.6s-16.4-6.2-22.6 0l-16 16c-6.2 6.2-6.2 16.4 0 22.6s16.4 6.2 22.6 0l16-16zm160-22.6c-6.2-6.2-16.4-6.2-22.6 0s-6.2 16.4 0 22.6l16 16c6.2 6.2 16.4 6.2 22.6 0s6.2-16.4 0-22.6l-16-16zM280 176c0-13.3-25.1-24-56-24s-56 10.7-56 24s25.1 24 56 24s56-10.7 56-24zM224 480C102.4 480 3.4 383.1 .1 262.2c13.2 16.4 29.8 30.7 48.2 42.5C94.7 334.6 156.9 352 224 352s129.3-17.4 175.7-47.3c18.5-11.9 35-26.2 48.2-42.5C444.6 383.1 345.6 480 224 480z";
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
    exports.faKiwiFruit = exports.definition;
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

// src/_entries/faKiwiFruit.js
var import_faKiwiFruit = __toESM(require_faKiwiFruit());
var export_faKiwiFruit = import_faKiwiFruit.definition;
export {
  export_faKiwiFruit as faKiwiFruit
};
