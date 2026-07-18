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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faDrone.js
var require_faDrone = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faDrone.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "drone";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f85f";
    var svgPathData = "M223 127.2L171.6 88.7C162.3 64.9 139.1 48 112 48c-35.3 0-64 28.7-64 64c0 27.1 16.9 50.3 40.7 59.6L127.2 223c-5 .7-10.1 1-15.2 1C50.1 224 0 173.9 0 112S50.1 0 112 0s112 50.1 112 112c0 5.2-.3 10.2-1 15.2zm-49.6 2.8l39.8 29.9 .1 .1 85.3 0 .1-.1 39.8-29.9 29.7-22.3C370.3 92.1 383.8 80 400 80c17.7 0 32 14.3 32 32c0 16.2-12.1 29.7-27.8 31.7l-22.3 29.7-29.9 39.8-.1 .1 0 85.3 .1 .1 29.9 39.8 22.3 29.7c15.7 2.1 27.8 15.5 27.8 31.7c0 17.7-14.3 32-32 32c-16.2 0-29.7-12.1-31.7-27.8l-29.7-22.3-39.8-29.9-.1-.1-85.3 0-.1 .1-39.8 29.9-29.7 22.3C141.7 419.9 128.2 432 112 432c-17.7 0-32-14.3-32-32c0-16.2 12.1-29.7 27.8-31.7l22.3-29.7 29.9-39.8 .1-.1 0-85.3-.1-.1-29.9-39.8-22.3-29.7C92.1 141.7 80 128.2 80 112c0-17.7 14.3-32 32-32c16.2 0 29.7 12.1 31.7 27.8l29.7 22.3zM224 400c0 61.9-50.1 112-112 112S0 461.9 0 400s50.1-112 112-112c5.2 0 10.2 .3 15.2 1L88.7 340.4C64.9 349.7 48 372.9 48 400c0 35.3 28.7 64 64 64c27.1 0 50.3-16.9 59.6-40.7L223 384.8c.7 5 1 10.1 1 15.2zm65-272.8c-.7-5-1-10.1-1-15.2C288 50.1 338.1 0 400 0s112 50.1 112 112s-50.1 112-112 112c-5.2 0-10.2-.3-15.2-1l38.5-51.3c23.8-9.3 40.7-32.5 40.7-59.6c0-35.3-28.7-64-64-64c-27.1 0-50.3 16.9-59.6 40.7L289 127.2zM288 400c0-5.2 .3-10.2 1-15.2l51.3 38.5c9.3 23.8 32.5 40.7 59.6 40.7c35.3 0 64-28.7 64-64c0-27.1-16.9-50.3-40.7-59.6L384.8 289c5-.7 10.1-1 15.2-1c61.9 0 112 50.1 112 112s-50.1 112-112 112s-112-50.1-112-112z";
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
    exports.faDrone = exports.definition;
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

// src/_entries/faDrone.js
var import_faDrone = __toESM(require_faDrone());
var export_faDrone = import_faDrone.definition;
export {
  export_faDrone as faDrone
};
