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

// src/node_modules/@fortawesome/free-solid-svg-icons/faKipSign.js
var require_faKipSign = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faKipSign.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "kip-sign";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "e1c4";
    var svgPathData = "M340.8 88.3c13.4-11.5 15-31.7 3.5-45.1s-31.7-15-45.1-3.5L128 186.4 128 64c0-17.7-14.3-32-32-32S64 46.3 64 64l0 168-32 0c-13.3 0-24 10.7-24 24s10.7 24 24 24l32 0 0 168c0 17.7 14.3 32 32 32s32-14.3 32-32l0-122.4 171.2 146.7c13.4 11.5 33.6 9.9 45.1-3.5s9.9-33.6-3.5-45.1L173.2 280 352 280c13.3 0 24-10.7 24-24s-10.7-24-24-24L173.2 232 340.8 88.3z";
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
    exports.faKipSign = exports.definition;
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

// src/_entries/faKipSign.js
var import_faKipSign = __toESM(require_faKipSign());
var export_faKipSign = import_faKipSign.definition;
export {
  export_faKipSign as faKipSign
};
