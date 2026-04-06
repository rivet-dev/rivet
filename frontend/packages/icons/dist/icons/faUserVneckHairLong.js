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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faUserVneckHairLong.js
var require_faUserVneckHairLong = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faUserVneckHairLong.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "user-vneck-hair-long";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e463";
    var svgPathData = "M304 128c0-11.4-2.4-22.2-6.7-32L296 96c-20.5 0-38.7-9.6-50.4-24.5C231.9 95.7 205.8 112 176 112l-30.4 0c-1 5.2-1.6 10.5-1.6 16l0 16c0 44.2 35.8 80 80 80s80-35.8 80-80l0-16zM96 128C96 57.3 153.3 0 224 0s128 57.3 128 128l0 11c0 33.9 13.5 66.5 37.5 90.5l3.9 3.9c4.2 4.2 6.6 10 6.6 16c0 12.5-10.1 22.6-22.6 22.6L224 272 70.6 272C58.1 272 48 261.9 48 249.4c0-6 2.4-11.8 6.6-16l3.9-3.9c24-24 37.5-56.6 37.5-90.5l0-11zm49.9 186.9L210.7 412c6.3 9.5 20.3 9.5 26.6 0l64.8-97.1c3.5-5.2 9.8-8 15.9-6.6c74.5 17.2 130 84 130 163.7l0 8c0 17.7-14.3 32-32 32L32 512c-17.7 0-32-14.3-32-32l0-8c0-79.7 55.5-146.5 130-163.7c6.1-1.4 12.4 1.4 15.9 6.6z";
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
    exports.faUserVneckHairLong = exports.definition;
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

// src/_entries/faUserVneckHairLong.js
var import_faUserVneckHairLong = __toESM(require_faUserVneckHairLong());
var export_faUserVneckHairLong = import_faUserVneckHairLong.definition;
export {
  export_faUserVneckHairLong as faUserVneckHairLong
};
