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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBottleBaby.js
var require_faBottleBaby = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBottleBaby.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bottle-baby";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e673";
    var svgPathData = "M220.6 49.7L224 48l0-16c0-17.7 14.3-32 32-32s32 14.3 32 32l0 16 3.4 1.7C308.9 58.5 320 76.4 320 96c17.7 0 32 14.3 32 32l0 32-192 0 0-32c0-17.7 14.3-32 32-32c0-19.6 11.1-37.5 28.6-46.3zM200 272l-84.4 0c4.5-19.6 13-38.1 25.2-54.4L160 192l192 0 19.2 25.6C389.9 242.5 400 272.8 400 304l0 144c0 35.3-28.7 64-64 64l-160 0c-35.3 0-64-28.7-64-64l0-16 88 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-88 0 0-64 88 0c13.3 0 24-10.7 24-24s-10.7-24-24-24z";
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
    exports.faBottleBaby = exports.definition;
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

// src/_entries/faBottleBaby.js
var import_faBottleBaby = __toESM(require_faBottleBaby());
var export_faBottleBaby = import_faBottleBaby.definition;
export {
  export_faBottleBaby as faBottleBaby
};
