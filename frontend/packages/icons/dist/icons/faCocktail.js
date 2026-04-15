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

// src/node_modules/@fortawesome/free-solid-svg-icons/faMartiniGlassCitrus.js
var require_faMartiniGlassCitrus = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faMartiniGlassCitrus.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "martini-glass-citrus";
    var width = 640;
    var height = 512;
    var aliases = ["cocktail"];
    var unicode = "f561";
    var svgPathData = "M576 80c0-44.2-35.8-80-80-80-18 0-34.6 6-48 16l-81 0c23.6-47.4 72.5-80 129-80 79.5 0 144 64.5 144 144S575.5 224 496 224c-6.5 0-13-.4-19.3-1.3l64-74.7c1.1-1.3 2.2-2.7 3.3-4.1 19.4-14.6 32-37.8 32-64zM66.9 82.6C72.2 71.3 83.5 64 96 64l384 0c12.5 0 23.8 7.3 29.1 18.6s3.4 24.7-4.8 34.2l-184.3 215 0 116.2 64 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-192 0c-17.7 0-32-14.3-32-32s14.3-32 32-32l64 0 0-116.2-184.3-215c-8.1-9.5-10-22.8-4.8-34.2zM165.6 128L288 270.8 410.4 128 165.6 128z";
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
    exports.faMartiniGlassCitrus = exports.definition;
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

// src/_entries/faCocktail.js
var import_faMartiniGlassCitrus = __toESM(require_faMartiniGlassCitrus());
var export_faCocktail = import_faMartiniGlassCitrus.definition;
export {
  export_faCocktail as faCocktail
};
