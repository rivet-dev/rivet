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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faHeadSideHeadphones.js
var require_faHeadSideHeadphones = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faHeadSideHeadphones.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "head-side-headphones";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f8c2";
    var svgPathData = "M256 0s0 0 0 0c95.2 0 174.2 69.3 189.4 160.1c2.2 13 6.7 25.7 15 36.1l42 52.6c6.2 7.8 9.6 17.4 9.6 27.4c0 24.2-19.6 43.8-43.8 43.8L448 320l0 64c0 35.3-28.7 64-64 64l-64 0 0 32c0 17.7-14.3 32-32 32L96 512c-17.7 0-32-14.3-32-32l0-72.7c0-16.7-6.9-32.5-17.1-45.8C16.6 322.4 0 274.1 0 224.2C0 122.8 67.4 36.9 160 9.3l0 77.3c0 12.4-7.4 23.3-17.4 30.5C114.4 137.4 96 170.6 96 208c0 61.9 50.1 112 112 112s112-50.1 112-112c0-37.4-18.4-70.6-46.6-90.9c-10-7.2-17.4-18.2-17.4-30.5L256 0zM240 208a32 32 0 1 0 -64 0 32 32 0 1 0 64 0zm-96 0a64 64 0 1 1 128 0 64 64 0 1 1 -128 0z";
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
    exports.faHeadSideHeadphones = exports.definition;
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

// src/_entries/faHeadSideHeadphones.js
var import_faHeadSideHeadphones = __toESM(require_faHeadSideHeadphones());
var export_faHeadSideHeadphones = import_faHeadSideHeadphones.definition;
export {
  export_faHeadSideHeadphones as faHeadSideHeadphones
};
