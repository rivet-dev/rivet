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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faUserDoctorHairLong.js
var require_faUserDoctorHairLong = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faUserDoctorHairLong.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "user-doctor-hair-long";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e459";
    var svgPathData = "M304 128c0-11.4-2.4-22.2-6.7-32L296 96c-20.5 0-38.7-9.6-50.4-24.5C231.9 95.7 205.8 112 176 112l-30.4 0c-1 5.2-1.6 10.5-1.6 16l0 16c0 44.2 35.8 80 80 80s80-35.8 80-80l0-16zM96 128C96 57.3 153.3 0 224 0s128 57.3 128 128l0 11c0 33.9 13.5 66.5 37.5 90.5l3.9 3.9c4.2 4.2 6.6 10 6.6 16c0 12.5-10.1 22.6-22.6 22.6L224 272 70.6 272C58.1 272 48 261.9 48 249.4c0-6 2.4-11.8 6.6-16l3.9-3.9c24-24 37.5-56.6 37.5-90.5l0-11zM288 320l0 42c-27.6 7.1-48 32.2-48 62l0 40c0 8.8 7.2 16 16 16l16 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l0-24c0-17.7 14.3-32 32-32s32 14.3 32 32l0 24c-8.8 0-16 7.2-16 16s7.2 16 16 16l16 0c8.8 0 16-7.2 16-16l0-40c0-29.8-20.4-54.9-48-62l0-38.6c73.1 15.3 128 80.2 128 157.8c0 17-13.8 30.7-30.7 30.7L30.7 512C13.8 512 0 498.2 0 481.3c0-77.7 54.9-142.5 128-157.8l0 46.9c-23.1 6.9-40 28.3-40 53.7c0 30.9 25.1 56 56 56s56-25.1 56-56c0-25.4-16.9-46.8-40-53.7l0-50.3 1.3 0 125.4 0 1.3 0zM120 424a24 24 0 1 1 48 0 24 24 0 1 1 -48 0z";
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
    exports.faUserDoctorHairLong = exports.definition;
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

// src/_entries/faUserDoctorHairLong.js
var import_faUserDoctorHairLong = __toESM(require_faUserDoctorHairLong());
var export_faUserDoctorHairLong = import_faUserDoctorHairLong.definition;
export {
  export_faUserDoctorHairLong as faUserDoctorHairLong
};
