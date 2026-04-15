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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFootballHelmet.js
var require_faFootballHelmet = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFootballHelmet.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "football-helmet";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f44f";
    var svgPathData = "M488.1 194.9c4.5 17.1-8.2 33.1-25.7 35.3L340.3 245.5l14 74.5L464 320c26.5 0 48 21.5 48 48l0 32 0 50.4c0 25.2-20.4 45.6-45.6 45.6l-17 0c-54.9 0-102-39.1-112.1-93.1L327.7 352 288 352l9.2 18.3c4.5 9 6.8 18.9 6.8 28.9c0 35.7-29 64.7-64.7 64.7l-1.7 0c-9 0-17.9-1.8-26.1-5.4l-93.4-40c-4-1.7-8.3-2.6-12.6-2.6l-14 0c-9.1 0-17.9-3.8-23.5-10.9C35.4 364.1 16 312.3 16 256C16 123.5 123.5 16 256 16c111.4 0 205.1 75.9 232.1 178.9zm-207.1 58c-14.3 1.8-25.1 14-25.1 28.4c0 4.4 1 8.8 3 12.8l13 26 49.7 0-13.2-70.6-27.4 3.4zM360.3 352l6 32L480 384l0-16c0-8.8-7.2-16-16-16l-103.7 0zM480 416l-105.3 0c13.1 28.7 42 48 74.7 48l17 0c7.5 0 13.6-6.1 13.6-13.6l0-34.4zM176 360a24 24 0 1 0 0-48 24 24 0 1 0 0 48z";
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
    exports.faFootballHelmet = exports.definition;
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

// src/_entries/faFootballHelmet.js
var import_faFootballHelmet = __toESM(require_faFootballHelmet());
var export_faFootballHelmet = import_faFootballHelmet.definition;
export {
  export_faFootballHelmet as faFootballHelmet
};
