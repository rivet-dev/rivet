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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceMask.js
var require_faFaceMask = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceMask.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-mask";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e37f";
    var svgPathData = "M410.5 281.7L541.8 222C525.1 96.7 417.9 0 288 0S50.9 96.7 34.2 222l131.3 59.7c8-15.3 24.1-25.7 42.5-25.7l160 0c18.4 0 34.5 10.4 42.5 25.7zm5.5 32.6l0 69.7 93.8 0c21.8-37.6 34.2-81.3 34.2-127.9L416 314.3zM413.3 416c-6.6 18.6-24.4 32-45.3 32l-160 0c-20.9 0-38.7-13.4-45.3-32l-74.6 0c46.9 58.5 119 96 199.9 96s152.9-37.5 199.9-96l-74.6 0zM160 384l0-69.7L32 256.1c0 46.6 12.5 90.3 34.2 127.9l93.8 0zm-4.7-180.7c-6.2 6.2-16.4 6.2-22.6 0s-6.2-16.4 0-22.6C146.4 167 184.6 144 240 144c8.8 0 16 7.2 16 16s-7.2 16-16 16c-47 0-77.1 19.7-84.7 27.3zM336 176c-8.8 0-16-7.2-16-16s7.2-16 16-16c55.4 0 93.6 23 107.3 36.7c6.2 6.2 6.2 16.4 0 22.6s-16.4 6.2-22.6 0C413.1 195.7 383 176 336 176zM208 304c-8.8 0-16 7.2-16 16s7.2 16 16 16l160 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-160 0zm0 64c-8.8 0-16 7.2-16 16s7.2 16 16 16l160 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-160 0z";
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
    exports.faFaceMask = exports.definition;
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

// src/_entries/faFaceMask.js
var import_faFaceMask = __toESM(require_faFaceMask());
var export_faFaceMask = import_faFaceMask.definition;
export {
  export_faFaceMask as faFaceMask
};
