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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faMoonStars.js
var require_faMoonStars = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faMoonStars.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "moon-stars";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f755";
    var svgPathData = "M284.8 99.2l11 38.6c1 3.6 4.4 6.2 8.2 6.2s7.1-2.5 8.2-6.2l11-38.6 38.6-11c3.6-1 6.2-4.4 6.2-8.2s-2.5-7.1-6.2-8.2l-38.6-11-11-38.6c-1-3.6-4.4-6.2-8.2-6.2s-7.1 2.5-8.2 6.2l-11 38.6-38.6 11c-3.6 1-6.2 4.4-6.2 8.2s2.5 7.1 6.2 8.2l38.6 11zm119 211.6c1.6 5.5 6.6 9.2 12.2 9.2s10.7-3.8 12.2-9.2l16.6-58 58-16.6c5.5-1.6 9.2-6.6 9.2-12.2s-3.8-10.7-9.2-12.2l-58-16.6-16.6-58c-1.6-5.5-6.6-9.2-12.2-9.2s-10.7 3.8-12.2 9.2l-16.6 58-58 16.6c-5.5 1.6-9.2 6.6-9.2 12.2s3.8 10.7 9.2 12.2l58 16.6 16.6 58zM191.8 128C85.8 128 0 214.1 0 320.2S85.8 512.4 191.8 512.4c52 0 99.1-20.7 133.6-54.4c5-4.9 6.3-12.5 3.1-18.7s-10.1-9.7-17-8.5c-8.1 1.4-16.5 2.2-25.1 2.2c-80.7 0-146.2-65.6-146.2-146.6c0-54.8 30-102.6 74.4-127.8c6.1-3.5 9.2-10.5 7.7-17.3s-7.3-11.9-14.3-12.5c-5.4-.5-10.8-.7-16.3-.7z";
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
    exports.faMoonStars = exports.definition;
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

// src/_entries/faMoonStars.js
var import_faMoonStars = __toESM(require_faMoonStars());
var export_faMoonStars = import_faMoonStars.definition;
export {
  export_faMoonStars as faMoonStars
};
