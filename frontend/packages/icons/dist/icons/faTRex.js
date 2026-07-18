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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTRex.js
var require_faTRex = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTRex.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "t-rex";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e629";
    var svgPathData = "M320 128l-41 0c-70.7 0-132.9 46.3-153.3 114l-5.9 19.8c-4.7 15.5-18.9 26.2-35.1 26.2C64.4 288 48 271.6 48 251.3l0-48.9c0-5.8-4.7-10.4-10.4-10.4c-3.5 0-6.7 1.7-8.7 4.6l-3.2 4.8C9 226.6 0 256.1 0 286.4C0 358 58 416 129.6 416l54.4 0 62.4 83.2c6 8.1 15.5 12.8 25.6 12.8l96 0c8.8 0 16-7.2 16-16c0-26.5-21.5-48-48-48l-18.6-49.6 28.4-25.6C359.9 360.1 368 342 368 323c0-37-30-67-67-67l-13 0c-8.8 0-16-7.2-16-16s7.2-16 16-16l13 0c35.1 0 66 18.3 83.5 45.8l.4-.3 43.5 29c2.2 1.5 3.6 4 3.6 6.7l0 14.9c0 13.3 10.7 24 24 24s24-10.7 24-24l0-14.9c0-18.7-9.4-36.2-24.9-46.6l-30.2-20.1L464 208l96 0c20.9 0 38.7-13.4 45.3-32l-40 0c-10.6 0-20.8-4.2-28.3-11.7L523.7 151c-4.5-4.5-10.6-7-17-7L488 144c-4.4 0-8-3.6-8-8s3.6-8 8-8l18.7 0c10.6 0 20.8 4.2 28.3 11.7L548.3 153c4.5 4.5 10.6 7 17 7l42.7 0 8 0c13.3 0 24-10.7 24-24l0-24c0-44.2-35.8-80-80-80l-18.7 0C534.7 13.4 516.9 0 496 0c-8.9 0-17.2 2.4-24.3 6.6C461.9 2.3 451.2 0 440.3 0c-33.6 0-63.5 21.5-74.2 53.4L352 96l-32 32zM480 56a16 16 0 1 1 32 0 16 16 0 1 1 -32 0z";
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
    exports.faTRex = exports.definition;
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

// src/_entries/faTRex.js
var import_faTRex = __toESM(require_faTRex());
var export_faTRex = import_faTRex.definition;
export {
  export_faTRex as faTRex
};
