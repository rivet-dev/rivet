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

// src/node_modules/@fortawesome/free-brands-svg-icons/faDartLang.js
var require_faDartLang = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faDartLang.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "dart-lang";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e693";
    var svgPathData = "M378.6 78.9c-2.8-.1-5.6-.2-8.5-.2l-264.1 0 143.2-72c7.4-4.4 18.8-6.7 30.4-6.7 13.5 0 29.4 9.2 37 16.8l62 62 0 .1zM107.3 96.5l262.8 0c16 0 25.4 1.4 35.4 9.3l106.5 106.4 0 208.8-79.3 .7-325.4-325.2zM96.5 373l0-262.2 323.8 323.8 .7 77.4-212.2 0-98.1-98.2C99.4 402.5 96.5 398.5 96.5 373zM78.7 105.3l0 267.7c0 3.3 .1 6.3 .2 9.1l-62-62C6.5 309.3 0 294.3 0 279.6 0 272.8 3.9 262.1 6.7 256l72-150.7z";
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
    exports.faDartLang = exports.definition;
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

// src/_entries/faDartLang.js
var import_faDartLang = __toESM(require_faDartLang());
var export_faDartLang = import_faDartLang.definition;
export {
  export_faDartLang as faDartLang
};
