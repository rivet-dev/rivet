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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faUserHoodie.js
var require_faUserHoodie = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faUserHoodie.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "user-hoodie";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e68a";
    var svgPathData = "M118.8 297.6c-1.3 10.6-8.6 19.5-18.2 24.2C41 350.7 0 411.7 0 482.3C0 498.7 13.3 512 29.7 512l388.6 0c16.4 0 29.7-13.3 29.7-29.7c0-70.6-41-131.6-100.6-160.5c-9.6-4.7-16.9-13.5-18.2-24.2c-1.2-9.6 2.7-19.1 9.4-26C366.7 242.8 384 203.4 384 160C384 71.6 312.4 0 224 0S64 71.6 64 160c0 43.4 17.3 82.8 45.4 111.6c6.8 6.9 10.6 16.4 9.4 26zM224 96c-36.2 0-64 28.3-64 61.3c0 17.5 8.5 35.8 22.9 53.6c13.1 16.2 29 29.5 41.1 38.5c12.1-8.9 28-22.3 41.1-38.5c14.4-17.9 22.9-36.1 22.9-53.6c0-33-27.8-61.3-64-61.3zm-96 61.3c0-51.5 43-93.3 96-93.3s96 41.8 96 93.3c0 55.6-53.3 100.9-80.5 120.4c-9.4 6.7-21.7 6.7-31.1 0C181.3 258.2 128 213 128 157.3zM160 400l0 64c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-64c0-8.8 7.2-16 16-16s16 7.2 16 16zm160 0l0 64c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-64c0-8.8 7.2-16 16-16s16 7.2 16 16z";
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
    exports.faUserHoodie = exports.definition;
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

// src/_entries/faUserHoodie.js
var import_faUserHoodie = __toESM(require_faUserHoodie());
var export_faUserHoodie = import_faUserHoodie.definition;
export {
  export_faUserHoodie as faUserHoodie
};
