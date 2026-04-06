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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faWineGlassCrack.js
var require_faWineGlassCrack = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faWineGlassCrack.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "wine-glass-crack";
    var width = 320;
    var height = 512;
    var aliases = ["fragile"];
    var unicode = "f4bb";
    var svgPathData = "M64 0C47.4 0 33.5 12.8 32.1 29.3l-14 168.4c-6 72 42.5 135.2 109.9 150.6l0 99.6-48 0c-17.7 0-32 14.3-32 32s14.3 32 32 32l80 0 80 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-48 0 0-99.6c67.4-15.4 115.9-78.6 109.9-150.6l-14-168.4C286.5 12.8 272.6 0 256 0L233.9 0C222 0 214.3 12.5 219.6 23.2l14.2 28.5c3.5 7.1 1.4 15.7-5.1 20.3l-58.4 40.9c-6 4.2-8.4 12-5.8 18.9l31.1 81c2.1 5.5-1.9 11.4-7.8 11.4c-2.4 0-4.7-1.1-6.3-2.9L92.1 118c-6.4-7.4-4.7-18.7 3.5-24l58.6-37.3c7.6-4.8 9.7-14.9 4.7-22.4L140.7 7.1c-3-4.5-8-7.1-13.3-7.1L64 0z";
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
    exports.faWineGlassCrack = exports.definition;
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

// src/_entries/faFragile.js
var import_faWineGlassCrack = __toESM(require_faWineGlassCrack());
var export_faFragile = import_faWineGlassCrack.definition;
export {
  export_faFragile as faFragile
};
