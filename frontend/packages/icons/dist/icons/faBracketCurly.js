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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBracketCurly.js
var require_faBracketCurly = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBracketCurly.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bracket-curly";
    var width = 256;
    var height = 512;
    var aliases = ["bracket-curly-left"];
    var unicode = "7b";
    var svgPathData = "M64 128c0-53 43-96 96-96l32 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-32 0c-17.7 0-32 14.3-32 32l0 37.5c0 25.5-10.1 49.9-28.1 67.9L77.3 256l22.6 22.6c18 18 28.1 42.4 28.1 67.9l0 37.5c0 17.7 14.3 32 32 32l32 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-32 0c-53 0-96-43-96-96l0-37.5c0-8.5-3.4-16.6-9.4-22.6L9.4 278.6c-12.5-12.5-12.5-32.8 0-45.3l45.3-45.3c6-6 9.4-14.1 9.4-22.6L64 128z";
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
    exports.faBracketCurly = exports.definition;
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

// src/_entries/faBracketCurly.js
var import_faBracketCurly = __toESM(require_faBracketCurly());
var export_faBracketCurly = import_faBracketCurly.definition;
export {
  export_faBracketCurly as faBracketCurly
};
