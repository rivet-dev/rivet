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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faWifiSlash.js
var require_faWifiSlash = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faWifiSlash.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "wifi-slash";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f6ac";
    var svgPathData = "M38.8 5.1C28.4-3.1 13.3-1.2 5.1 9.2S-1.2 34.7 9.2 42.9l592 464c10.4 8.2 25.5 6.3 33.7-4.1s6.3-25.5-4.1-33.7L364.6 260.4c39.2 7.9 74.7 26.1 103.6 51.6c13.3 11.7 33.5 10.4 45.2-2.8s10.4-33.5-2.8-45.2C459.8 219.2 393 192 320 192c-13.3 0-26.5 .9-39.3 2.7l-95.1-74.5C227.4 104.5 272.7 96 320 96c103.2 0 196.8 40.7 265.8 106.9c12.8 12.2 33 11.8 45.2-.9s11.8-33-.9-45.2C549.7 79.5 440.4 32 320 32c-68.7 0-133.7 15.5-191.9 43.1L38.8 5.1zM380.8 396l-55.6-43.8c-1.7-.1-3.4-.2-5.2-.2c-35.3 0-64 28.7-64 64s28.7 64 64 64s64-28.7 64-64c0-7-1.1-13.7-3.2-20zM228 275.7L173 232.3c-15.5 9.2-30.1 19.8-43.5 31.7c-13.3 11.7-14.5 31.9-2.8 45.2s31.9 14.5 45.2 2.8c16.7-14.8 35.6-27 56.2-36.3zM93.6 169.8L41.7 128.9C30.7 137.7 20 147 9.8 156.7C-2.9 169-3.3 189.2 8.9 202s32.5 13.2 45.2 .9c12.4-11.9 25.6-23 39.5-33.1z";
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
    exports.faWifiSlash = exports.definition;
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

// src/_entries/faWifiSlash.js
var import_faWifiSlash = __toESM(require_faWifiSlash());
var export_faWifiSlash = import_faWifiSlash.definition;
export {
  export_faWifiSlash as faWifiSlash
};
