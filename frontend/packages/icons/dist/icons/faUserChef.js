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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faUserChef.js
var require_faUserChef = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faUserChef.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "user-chef";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e3d2";
    var svgPathData = "M384 48c0 20.9-13.4 38.7-32 45.3l0 18.7 0 64c0 70.7-57.3 128-128 128s-128-57.3-128-128l0-64 0-18.7C77.4 86.7 64 68.9 64 48C64 21.5 85.5 0 112 0c14.3 0 27.2 6.3 36 16.3C156.8 6.3 169.7 0 184 0c16.7 0 31.4 8.5 40 21.5C232.6 8.5 247.3 0 264 0c14.3 0 27.2 6.3 36 16.3C308.8 6.3 321.7 0 336 0c26.5 0 48 21.5 48 48zM304 176l0-16-160 0 0 16c0 44.2 35.8 80 80 80s80-35.8 80-80zM178.4 340.6c7.5 1.9 8.2 12.3 1 15.2l-43.2 17.3C111.9 382.8 96 406.3 96 432.5L96 480l0 24 0 8-8 0-24 0-38.4 0C11.5 512 0 500.5 0 486.4c0-58.8 37.7-108.8 90.3-127c3.3-1.2 5.7-4.3 5.7-7.8l0-21.4c0-5.2 4.9-9 9.9-7.8l72.5 18.1zM352 512l-192 0-24 0-8 0 0-8 0-24 0-47.5c0-13.1 8-24.9 20.1-29.7L341 325.6c5.3-2.1 11 1.8 11 7.4l0 18.6c0 3.5 2.3 6.6 5.7 7.8c52.6 18.2 90.3 68.2 90.3 127c0 14.1-11.5 25.6-25.6 25.6L352 512zM224 432a16 16 0 1 0 -32 0 16 16 0 1 0 32 0zm80 16a16 16 0 1 0 0-32 16 16 0 1 0 0 32z";
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
    exports.faUserChef = exports.definition;
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

// src/_entries/faUserChef.js
var import_faUserChef = __toESM(require_faUserChef());
var export_faUserChef = import_faUserChef.definition;
export {
  export_faUserChef as faUserChef
};
