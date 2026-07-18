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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faUpToBracket.js
var require_faUpToBracket = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faUpToBracket.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "up-to-bracket";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e66e";
    var svgPathData = "M374.1 294.1L251.3 171.3c-7.2-7.2-17.1-11.3-27.3-11.3s-20.1 4.1-27.3 11.3L73.9 294.1c-6.4 6.4-9.9 15-9.9 24C64 336.8 79.2 352 97.9 352l62.1 0 0 128c0 17.7 14.3 32 32 32l64 0c17.7 0 32-14.3 32-32l0-128 62.1 0c18.7 0 33.9-15.2 33.9-33.9c0-9-3.6-17.6-9.9-24zM64 160l0-64c0-17.7 14.3-32 32-32l256 0c17.7 0 32 14.3 32 32l0 64c0 17.7 14.3 32 32 32s32-14.3 32-32l0-64c0-53-43-96-96-96L96 0C43 0 0 43 0 96l0 64c0 17.7 14.3 32 32 32s32-14.3 32-32z";
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
    exports.faUpToBracket = exports.definition;
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

// src/_entries/faUpToBracket.js
var import_faUpToBracket = __toESM(require_faUpToBracket());
var export_faUpToBracket = import_faUpToBracket.definition;
export {
  export_faUpToBracket as faUpToBracket
};
