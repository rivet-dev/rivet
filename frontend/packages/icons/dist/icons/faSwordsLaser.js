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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSwordsLaser.js
var require_faSwordsLaser = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSwordsLaser.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "swords-laser";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e03d";
    var svgPathData = "M135 7c-9.1 9.1-9.4 23.7-.7 33.2L360 286.9l0-105.1L168.2 6.3C158.7-2.4 144.1-2.1 135 7zm263 349.7l49 49 27.3-27.3 32-32L533.7 319 411.3 196.7c-4.6-4.6-11.5-5.9-17.4-3.5s-9.9 8.3-9.9 14.8l0 114.7c0 12.7 5.1 24.9 14.1 33.9zM523.3 352L629 457.7l3.5-3.5c4.8-4.8 7.5-11.3 7.5-18.1s-2.7-13.3-7.5-18.1L545 330.3 523.3 352zM512 363.3L491.3 384 597 489.7 617.7 469 512 363.3zm-32 32L458.3 417l87.5 87.5c4.8 4.8 11.3 7.5 18.1 7.5s13.3-2.7 18.1-7.5l3.5-3.5L480 395.3zM247.6 211.4l-24.7 22.6c-1.6-3.9-4.7-7.2-8.7-8.8c-6-2.5-12.9-1.1-17.4 3.5l-25.4 25.4-4.9-4.9c-12.5-12.3-32.6-12.2-45 .2c-12.5 12.5-12.5 32.8 0 45.3l4.7 4.7L7.5 417.9C2.7 422.7 0 429.2 0 436s2.7 13.3 7.5 18.1l50.3 50.3c4.8 4.8 11.3 7.5 18.1 7.5s13.3-2.7 18.1-7.5L209.9 388.7c9-9 14.1-21.2 14.1-33.9l0-6.6 74.3-81.2-50.7-55.5zm142.8-45.2l115.3-126c8.7-9.5 8.3-24.1-.7-33.2s-23.7-9.4-33.2-.7L343.7 123.5l46.7 42.7zM152 344a16 16 0 1 1 32 0 16 16 0 1 1 -32 0z";
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
    exports.faSwordsLaser = exports.definition;
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

// src/_entries/faSwordsLaser.js
var import_faSwordsLaser = __toESM(require_faSwordsLaser());
var export_faSwordsLaser = import_faSwordsLaser.definition;
export {
  export_faSwordsLaser as faSwordsLaser
};
