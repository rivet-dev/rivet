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

// src/node_modules/@fortawesome/free-solid-svg-icons/faWonSign.js
var require_faWonSign = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faWonSign.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "won-sign";
    var width = 512;
    var height = 512;
    var aliases = [8361, "krw", "won"];
    var unicode = "f159";
    var svgPathData = "M62.4 53.9C56.8 37.1 38.7 28.1 21.9 33.6S-3.9 57.4 1.7 74.1L56.9 240 32 240c-13.3 0-24 10.7-24 24s10.7 24 24 24l40.9 0 56.7 170.1c4.5 13.5 17.4 22.4 31.6 21.9s26.4-10.4 29.8-24.2L233 288 279 288 321 455.8c3.4 13.8 15.6 23.7 29.8 24.2s27.1-8.4 31.6-21.9L439.1 288 480 288c13.3 0 24-10.7 24-24s-10.7-24-24-24l-24.9 0 55.3-165.9c5.6-16.8-3.5-34.9-20.2-40.5s-34.9 3.5-40.5 20.2l-62 186.1-54.6 0-45.9-183.8C283.5 42 270.7 32 256 32s-27.5 10-31 24.2L179 240 124.4 240 62.4 53.9zm78 234.1l26.6 0-11.4 45.6-15.2-45.6zM245 240l11-44.1 11 44.1-22 0zm100 48l26.6 0-15.2 45.6-11.4-45.6z";
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
    exports.faWonSign = exports.definition;
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

// src/_entries/faWonSign.js
var import_faWonSign = __toESM(require_faWonSign());
var export_faWonSign = import_faWonSign.definition;
export {
  export_faWonSign as faWonSign
};
