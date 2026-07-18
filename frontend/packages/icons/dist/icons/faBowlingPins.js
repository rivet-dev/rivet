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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBowlingPins.js
var require_faBowlingPins = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBowlingPins.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bowling-pins";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f437";
    var svgPathData = "M128 0c35.3 0 64 28.7 64 64l0 1.1c0 7.4-1 14.8-2.9 22l-17.7 64.8c-.7 2.7-1.3 5.4-1.8 8.1l-83.1 0c-.5-2.7-1.1-5.4-1.8-8.1L66.9 87.1c-2-7.2-2.9-14.6-2.9-22L64 64C64 28.7 92.7 0 128 0zM82.1 210.4c2.2-6 3.8-12.1 4.8-18.4l82.3 0c1 6.3 2.6 12.4 4.8 18.4l38.6 104.7c7.6 20.6 11.5 42.5 11.5 64.4c0 28.9-6.7 57.5-19.7 83.4l-15.7 31.4C183.2 505.2 172.1 512 160 512l-64 0c-12.1 0-23.2-6.8-28.6-17.7L51.7 462.9C38.7 437 32 408.5 32 379.5c0-22 3.9-43.8 11.5-64.4L82.1 210.4zM384 0c35.3 0 64 28.7 64 64l0 1.1c0 7.4-1 14.8-2.9 22l-17.7 64.8c-.7 2.7-1.3 5.4-1.8 8.1l-83.1 0c-.5-2.7-1.1-5.4-1.8-8.1L322.9 87.1c-2-7.2-2.9-14.6-2.9-22l0-1.1c0-35.3 28.7-64 64-64zM338.1 210.4c2.2-6 3.8-12.1 4.8-18.4l82.3 0c1 6.3 2.6 12.4 4.8 18.4L470 319.3c6.6 17.9 10 36.8 10 55.8c0 20.5-3.9 40.9-11.5 59.9l-22.7 56.9C440.9 504 429.1 512 416 512l-64 0c-13.1 0-24.9-8-29.7-20.1L299.5 435C291.9 416 288 395.6 288 375.1c0-19 3.4-37.9 10-55.8l40.1-108.9z";
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
    exports.faBowlingPins = exports.definition;
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

// src/_entries/faBowlingPins.js
var import_faBowlingPins = __toESM(require_faBowlingPins());
var export_faBowlingPins = import_faBowlingPins.definition;
export {
  export_faBowlingPins as faBowlingPins
};
