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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceWoozy.js
var require_faFaceWoozy = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceWoozy.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-woozy";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e3a2";
    var svgPathData = "M256 512A256 256 0 1 0 256 0a256 256 0 1 0 0 512zM411.3 347.3L372 386.6c-13.7 13.7-32.3 21.4-51.7 21.4c-13.7 0-27.1-3.8-38.7-11.1L267.4 388c-18.2-11.4-40.8-13.4-60.7-5.4c-30.4 12.2-64.8 8.6-92-9.6l-11.5-7.7c-7.4-4.9-9.3-14.8-4.4-22.2s14.8-9.3 22.2-4.4l11.5 7.7c18.5 12.3 41.8 14.7 62.4 6.5c29.4-11.8 62.7-8.8 89.6 8l14.2 8.9c6.5 4.1 14.1 6.2 21.8 6.2c10.9 0 21.3-4.3 29-12l39.3-39.3c6.2-6.2 16.4-6.2 22.6 0s6.2 16.4 0 22.6zM200.9 98.7c7.4 4.9 9.3 14.8 4.4 22.2c-28 42-69.6 64.1-88.3 70.3c-8.4 2.8-17.4-1.7-20.2-10.1s1.7-17.4 10.1-20.2c13.4-4.5 48.5-22.9 71.7-57.7c4.9-7.4 14.8-9.3 22.2-4.4zm22.6 121.4c2.1 8.6-3.1 17.3-11.6 19.4l-64 16c-8.6 2.1-17.3-3.1-19.4-11.6s3.1-17.3 11.6-19.4l64-16c8.6-2.1 17.3 3.1 19.4 11.6zM336 272c-17.7 0-32-28.7-32-64s14.3-64 32-64s32 28.7 32 64s-14.3 64-32 64z";
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
    exports.faFaceWoozy = exports.definition;
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

// src/_entries/faFaceWoozy.js
var import_faFaceWoozy = __toESM(require_faFaceWoozy());
var export_faFaceWoozy = import_faFaceWoozy.definition;
export {
  export_faFaceWoozy as faFaceWoozy
};
