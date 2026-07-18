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

// src/node_modules/@fortawesome/free-brands-svg-icons/faPiedPiperPp.js
var require_faPiedPiperPp = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faPiedPiperPp.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "pied-piper-pp";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f1a7";
    var svgPathData = "M205.3 174.6c0 21.1-14.2 38.1-31.7 38.1-7.1 0-12.8-1.2-17.2-3.7l0-68c4.4-2.7 10.1-4.2 17.2-4.2 17.5 0 31.7 16.9 31.7 37.8zm52.6 67c-7.1 0-12.8 1.5-17.2 4.2l0 68c4.4 2.5 10.1 3.7 17.2 3.7 17.4 0 31.7-16.9 31.7-37.8 0-21.1-14.3-38.1-31.7-38.1zM448 80l0 352c0 26.5-21.5 48-48 48L48 480c-26.5 0-48-21.5-48-48L0 80C0 53.5 21.5 32 48 32l352 0c26.5 0 48 21.5 48 48zM185 255.1c41 0 74.2-35.6 74.2-79.6S226 95.9 185 95.9c-12 0-24.1 3.2-34.6 8.8l-45.7 0 0 206.3 51.8-10.1 0-50.6c8.6 3.1 18.1 4.8 28.5 4.8zm158.4 25.3c0-44-33.2-79.6-73.9-79.6-3.2 0-6.4 .2-9.6 .7-3.7 12.5-10.1 23.8-19.2 33.4-13.8 15-32.2 23.8-51.8 24.8l0 156.3 51.8-10.1 0-50.6c8.6 3.2 18.2 4.7 28.7 4.7 40.8 0 74-35.6 74-79.6z";
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
    exports.faPiedPiperPp = exports.definition;
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

// src/_entries/faPiedPiperPp.js
var import_faPiedPiperPp = __toESM(require_faPiedPiperPp());
var export_faPiedPiperPp = import_faPiedPiperPp.definition;
export {
  export_faPiedPiperPp as faPiedPiperPp
};
