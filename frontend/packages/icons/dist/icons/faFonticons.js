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

// src/node_modules/@fortawesome/free-brands-svg-icons/faFonticons.js
var require_faFonticons = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faFonticons.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "fonticons";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f280";
    var svgPathData = "M0 32l0 448 448 0 0-448-448 0zM187 172.9c-18.4 0-19 9.9-19 27.4l0 23.3c0 2.4-3.5 4.4-.6 4.4l67.4 0-11.1 37.3-55.7 0 0 112.9c0 5.8-2 6.7 3.2 7.3l43.5 4.1 0 25.1-130.7 0 0-25.7 21.3-2c5.2-.6 6.7-2.3 6.7-7.9l0-111.4c0-2.3-2.9-2.3-5.8-2.3l-22.2 0 0-37.4 28 0 0-21c0-49.6 26.5-70 77.3-70 34.1 0 64.7 8.2 64.7 52.8l-50.7 6.1c.3-18.7-4.4-23-16.3-23zm74.3 241.8l0-25.1 20.4-2.6c5.2-.6 7.6-1.7 7.6-7.3l0-107.9c0-4.1-2.9-6.7-6.7-7.9l-24.2-6.4 6.7-29.5 80.2 0 0 151.7c0 5.8-2.6 6.4 2.9 7.3l15.7 2.6 0 25.1-102.6 0zm80.8-255.5l9 33.2-7.3 7.3-31.2-16.6-31.2 16.6-7.3-7.3 9-33.2-21.8-24.2 3.5-9.6 27.7 0 15.5-28 9.3 0 15.5 28 27.7 0 3.5 9.6-21.9 24.2z";
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
    exports.faFonticons = exports.definition;
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

// src/_entries/faFonticons.js
var import_faFonticons = __toESM(require_faFonticons());
var export_faFonticons = import_faFonticons.definition;
export {
  export_faFonticons as faFonticons
};
