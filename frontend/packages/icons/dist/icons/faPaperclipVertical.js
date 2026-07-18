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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPaperclipVertical.js
var require_faPaperclipVertical = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPaperclipVertical.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "paperclip-vertical";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "e3c2";
    var svgPathData = "M32 128C32 57.3 89.3 0 160 0s128 57.3 128 128l0 192c0 44.2-35.8 80-80 80s-80-35.8-80-80l0-160c0-17.7 14.3-32 32-32s32 14.3 32 32l0 160c0 8.8 7.2 16 16 16s16-7.2 16-16l0-192c0-35.3-28.7-64-64-64s-64 28.7-64 64l0 208c0 61.9 50.1 112 112 112s112-50.1 112-112l0-176c0-17.7 14.3-32 32-32s32 14.3 32 32l0 176c0 97.2-78.8 176-176 176s-176-78.8-176-176l0-208z";
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
    exports.faPaperclipVertical = exports.definition;
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

// src/_entries/faPaperclipVertical.js
var import_faPaperclipVertical = __toESM(require_faPaperclipVertical());
var export_faPaperclipVertical = import_faPaperclipVertical.definition;
export {
  export_faPaperclipVertical as faPaperclipVertical
};
