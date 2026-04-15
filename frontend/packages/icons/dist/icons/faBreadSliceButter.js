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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBreadSliceButter.js
var require_faBreadSliceButter = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBreadSliceButter.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bread-slice-butter";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e3e1";
    var svgPathData = "M0 192C0 64 192 32 256 32s256 32 256 160c0 35.3-28.7 64-64 64l0 176c0 26.5-21.5 48-48 48l-288 0c-26.5 0-48-21.5-48-48l0-176c-35.3 0-64-28.7-64-64zm182.6 73.4c-12.5 12.5-12.5 32.8 0 45.3l50.7 50.7c12.5 12.5 32.8 12.5 45.3 0l50.7-50.7c12.5-12.5 12.5-32.8 0-45.3l-50.7-50.7c-12.5-12.5-32.8-12.5-45.3 0l-50.7 50.7z";
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
    exports.faBreadSliceButter = exports.definition;
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

// src/_entries/faBreadSliceButter.js
var import_faBreadSliceButter = __toESM(require_faBreadSliceButter());
var export_faBreadSliceButter = import_faBreadSliceButter.definition;
export {
  export_faBreadSliceButter as faBreadSliceButter
};
