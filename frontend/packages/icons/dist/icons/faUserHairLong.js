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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faUserHairLong.js
var require_faUserHairLong = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faUserHairLong.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "user-hair-long";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e45b";
    var svgPathData = "M304 128c0-11.4-2.4-22.2-6.7-32L296 96c-20.5 0-38.7-9.6-50.4-24.5C231.9 95.7 205.8 112 176 112l-30.4 0c-1 5.2-1.6 10.5-1.6 16l0 16c0 44.2 35.8 80 80 80s80-35.8 80-80l0-16zM96 128C96 57.3 153.3 0 224 0s128 57.3 128 128l0 11c0 33.9 13.5 66.5 37.5 90.5l3.9 3.9c4.2 4.2 6.6 10 6.6 16c0 12.5-10.1 22.6-22.6 22.6L224 272 70.6 272C58.1 272 48 261.9 48 249.4c0-6 2.4-11.8 6.6-16l3.9-3.9c24-24 37.5-56.6 37.5-90.5l0-11zm65.3 192l125.4 0C375.8 320 448 392.2 448 481.3c0 17-13.8 30.7-30.7 30.7L30.7 512C13.8 512 0 498.2 0 481.3C0 392.2 72.2 320 161.3 320z";
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
    exports.faUserHairLong = exports.definition;
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

// src/_entries/faUserHairLong.js
var import_faUserHairLong = __toESM(require_faUserHairLong());
var export_faUserHairLong = import_faUserHairLong.definition;
export {
  export_faUserHairLong as faUserHairLong
};
