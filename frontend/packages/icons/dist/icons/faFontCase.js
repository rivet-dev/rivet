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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFontCase.js
var require_faFontCase = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFontCase.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "font-case";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f866";
    var svgPathData = "M206 52.8C201.3 40.3 189.3 32 176 32s-25.3 8.3-30 20.8L2 436.8c-6.2 16.5 2.2 35 18.7 41.2s35-2.2 41.2-18.7L96.2 368l159.6 0L290 459.2c6.2 16.5 24.7 24.9 41.2 18.7s24.9-24.6 18.7-41.2L206 52.8zM231.8 304l-111.6 0L176 155.1 231.8 304zM608 160c-13 0-24.1 7.7-29.2 18.8C559.4 166.9 536.5 160 512 160c-70.7 0-128 57.3-128 128l0 64c0 70.7 57.3 128 128 128c24.5 0 47.4-6.9 66.8-18.8c5 11.1 16.2 18.8 29.2 18.8c17.7 0 32-14.3 32-32l0-96 0-64 0-96c0-17.7-14.3-32-32-32zM576 288l0 64c0 35.3-28.7 64-64 64s-64-28.7-64-64l0-64c0-35.3 28.7-64 64-64s64 28.7 64 64z";
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
    exports.faFontCase = exports.definition;
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

// src/_entries/faFontCase.js
var import_faFontCase = __toESM(require_faFontCase());
var export_faFontCase = import_faFontCase.definition;
export {
  export_faFontCase as faFontCase
};
