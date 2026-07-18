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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faUserHairBuns.js
var require_faUserHairBuns = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faUserHairBuns.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "user-hair-buns";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e3d3";
    var svgPathData = "M304 128l0 16c0 44.2-35.8 80-80 80s-80-35.8-80-80l0-16c0-5.5 .6-10.8 1.6-16l14.4 0c26.2 0 49.4-12.6 64-32c14.6 19.4 37.8 32 64 32l14.4 0c1 5.2 1.6 10.5 1.6 16zM84 192c7 0 13.7-1.6 19.6-4.6c17.8 49.3 65 84.6 120.4 84.6s102.7-35.3 120.4-84.6c5.9 2.9 12.5 4.6 19.6 4.6c24.3 0 44-19.7 44-44l0-40c0-24.3-19.7-44-44-44c-9.3 0-17.9 2.9-25 7.8C318.2 29.3 274.5 0 224 0s-94.2 29.3-115 71.8C101.9 66.9 93.3 64 84 64c-24.3 0-44 19.7-44 44l0 40c0 24.3 19.7 44 44 44zm77.3 128C72.2 320 0 392.2 0 481.3c0 17 13.8 30.7 30.7 30.7l386.6 0c17 0 30.7-13.8 30.7-30.7C448 392.2 375.8 320 286.7 320l-125.4 0z";
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
    exports.faUserHairBuns = exports.definition;
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

// src/_entries/faUserHairBuns.js
var import_faUserHairBuns = __toESM(require_faUserHairBuns());
var export_faUserHairBuns = import_faUserHairBuns.definition;
export {
  export_faUserHairBuns as faUserHairBuns
};
