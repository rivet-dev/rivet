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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faMusicNoteSlash.js
var require_faMusicNoteSlash = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faMusicNoteSlash.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "music-note-slash";
    var width = 640;
    var height = 512;
    var aliases = ["music-alt-slash"];
    var unicode = "f8d0";
    var svgPathData = "M512 32c0-9.9-4.5-19.2-12.3-25.2S481.8-1.4 472.2 1l-128 32C330 36.5 320 49.3 320 64l0 64 0 97.5L38.8 5.1C28.4-3.1 13.3-1.2 5.1 9.2S-1.2 34.7 9.2 42.9l592 464c10.4 8.2 25.5 6.3 33.7-4.1s6.3-25.5-4.1-33.7L384 275.7 384 153 487.8 127C502 123.5 512 110.7 512 96l0-64zM272 352c-61.9 0-112 35.8-112 80s50.1 80 112 80s112-35.8 112-80l0-20.8L317.8 359c-14-4.5-29.5-7-45.8-7z";
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
    exports.faMusicNoteSlash = exports.definition;
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

// src/_entries/faMusicAltSlash.js
var import_faMusicNoteSlash = __toESM(require_faMusicNoteSlash());
var export_faMusicAltSlash = import_faMusicNoteSlash.definition;
export {
  export_faMusicAltSlash as faMusicAltSlash
};
