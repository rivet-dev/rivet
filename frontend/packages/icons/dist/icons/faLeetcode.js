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

// src/node_modules/@fortawesome/free-brands-svg-icons/faLeetcode.js
var require_faLeetcode = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faLeetcode.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "leetcode";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e86a";
    var svgPathData = "M235.4 9.3C246.5-2.5 265.1-3.2 277 7.9s12.5 29.8 1.4 41.6l-43 46c21.5 3.3 42.1 11.8 59.1 25.6l74.6 60.3c12.6 10.2 14.6 28.8 4.4 41.4s-28.7 14.6-41.4 4.4l-74.6-60.3c-26.2-21.1-71.9-18.2-94.5 5.9L81 261c-21.5 23-19.1 60 5.6 84.2l91.2 89.4c24.9 24 65.6 24 90.1-.4l51.1-51c11.5-11.5 30.1-11.5 41.6 .1s11.4 30.2-.1 41.7l-51.1 51c-47.1 47.1-124 47.7-171.9 1.6l-92-90.2C-1.4 341.3-6 267.9 38 220.7L235.4 9.3zM411.7 273.2c16.2 0 29.4 13.2 29.4 29.5s-13.2 29.5-29.4 29.5l-216.9 0c-16.2 0-29.4-13.2-29.4-29.5s13.2-29.5 29.4-29.5l216.9 0z";
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
    exports.faLeetcode = exports.definition;
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

// src/_entries/faLeetcode.js
var import_faLeetcode = __toESM(require_faLeetcode());
var export_faLeetcode = import_faLeetcode.definition;
export {
  export_faLeetcode as faLeetcode
};
