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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faEmptySet.js
var require_faEmptySet = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faEmptySet.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "empty-set";
    var width = 512;
    var height = 512;
    var aliases = [8709, 216];
    var unicode = "f656";
    var svgPathData = "M435.4 121.9l67.2-67.2c12.5-12.5 12.5-32.8 0-45.3s-32.8-12.5-45.3 0L390.2 76.6C352.8 48.6 306.3 32 256 32C132.3 32 32 132.3 32 256c0 50.3 16.6 96.8 44.6 134.2L9.4 457.4c-12.5 12.5-12.5 32.8 0 45.3s32.8 12.5 45.3 0l67.2-67.2c37.4 28 83.8 44.6 134.1 44.6c123.7 0 224-100.3 224-224c0-50.3-16.6-96.8-44.6-134.1zm-91.2 .7L122.5 344.2C105.8 318.9 96 288.6 96 256c0-88.4 71.6-160 160-160c32.6 0 62.9 9.8 88.2 26.5zm-176.5 267L389.5 167.8C406.2 193.1 416 223.4 416 256c0 88.4-71.6 160-160 160c-32.6 0-62.9-9.8-88.2-26.5z";
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
    exports.faEmptySet = exports.definition;
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

// src/_entries/faEmptySet.js
var import_faEmptySet = __toESM(require_faEmptySet());
var export_faEmptySet = import_faEmptySet.definition;
export {
  export_faEmptySet as faEmptySet
};
