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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faHatWinter.js
var require_faHatWinter = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faHatWinter.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "hat-winter";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f7a8";
    var svgPathData = "M279.9 22.2C279 9.8 268.7 0 256 0s-23 9.8-23.9 22.2c-9.4-8.1-23.7-7.7-32.6 1.2s-9.4 23.2-1.2 32.6C185.8 57 176 67.3 176 80s9.8 23 22.2 23.9c-8.1 9.4-7.7 23.7 1.2 32.6s23.2 9.4 32.6 1.2C233 150.2 243.3 160 256 160s23-9.8 23.9-22.2c9.4 8.1 23.7 7.7 32.6-1.2s9.4-23.2 1.2-32.6C326.2 103 336 92.7 336 80s-9.8-23-22.2-23.9c8.1-9.4 7.7-23.7-1.2-32.6s-23.2-9.4-32.6-1.2zM0 480c0 17.7 14.3 32 32 32l448 0c17.7 0 32-14.3 32-32s-14.3-32-32-32L32 448c-17.7 0-32 14.3-32 32zM216.4 175.6c-14.2 0-28.4-5.4-39.3-16.1c-62.7 27.1-98.6 85.4-118.9 139.6c4.2 1.6 8.9 1.4 13-.6l35.4-17.7c13.5-6.8 29.4-6.8 42.9 0l35.4 17.7c4.5 2.3 9.8 2.3 14.3 0l35.4-17.7c13.5-6.8 29.4-6.8 42.9 0l35.4 17.7c4.5 2.3 9.8 2.3 14.3 0l35.4-17.7c13.5-6.8 29.4-6.8 42.9 0l35.4 17.7c4.1 2 8.8 2.2 13 .6c-20.3-54.2-56.2-112.5-118.9-139.6c-10.9 10.7-25.1 16.1-39.3 16.1C285.5 185.7 271.5 192 256 192s-29.5-6.3-39.6-16.4zM48.1 329.5C34.3 377.5 32 416 32 416l448 0s-2.3-38.5-16.1-86.5c-12.2 4.3-25.6 3.5-37.3-2.4l-35.4-17.7c-4.5-2.3-9.8-2.3-14.3 0l-35.4 17.7c-13.5 6.8-29.4 6.8-42.9 0l-35.4-17.7c-4.5-2.3-9.8-2.3-14.3 0l-35.4 17.7c-13.5 6.8-29.4 6.8-42.9 0l-35.4-17.7c-4.5-2.3-9.8-2.3-14.3 0L85.5 327.2c-11.7 5.8-25.2 6.6-37.3 2.4z";
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
    exports.faHatWinter = exports.definition;
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

// src/_entries/faHatWinter.js
var import_faHatWinter = __toESM(require_faHatWinter());
var export_faHatWinter = import_faHatWinter.definition;
export {
  export_faHatWinter as faHatWinter
};
