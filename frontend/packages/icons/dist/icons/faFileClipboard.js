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

// src/node_modules/@fortawesome/free-solid-svg-icons/faPaste.js
var require_faPaste = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faPaste.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "paste";
    var width = 512;
    var height = 512;
    var aliases = ["file-clipboard"];
    var unicode = "f0ea";
    var svgPathData = "M64 0C28.7 0 0 28.7 0 64L0 384c0 35.3 28.7 64 64 64l112 0 0-224c0-61.9 50.1-112 112-112l64 0 0-48c0-35.3-28.7-64-64-64L64 0zM248 112l-144 0c-13.3 0-24-10.7-24-24s10.7-24 24-24l144 0c13.3 0 24 10.7 24 24s-10.7 24-24 24zm40 48c-35.3 0-64 28.7-64 64l0 224c0 35.3 28.7 64 64 64l160 0c35.3 0 64-28.7 64-64l0-165.5c0-17-6.7-33.3-18.7-45.3l-58.5-58.5c-12-12-28.3-18.7-45.3-18.7L288 160z";
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
    exports.faPaste = exports.definition;
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

// src/_entries/faFileClipboard.js
var import_faPaste = __toESM(require_faPaste());
var export_faFileClipboard = import_faPaste.definition;
export {
  export_faFileClipboard as faFileClipboard
};
