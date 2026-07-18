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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCucumber.js
var require_faCucumber = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCucumber.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "cucumber";
    var width = 512;
    var height = 512;
    var aliases = [129362];
    var unicode = "e401";
    var svgPathData = "M37.5 474.5c47.8 47.8 124 49.9 174.3 6.3c12.6 10.3 31.1 9.6 42.9-2.2c11.6-11.6 12.4-30 2.4-42.6L436 257c12.6 10.1 31 9.3 42.6-2.4c11.7-11.7 12.5-30.3 2.2-42.9c43.6-50.3 41.5-126.5-6.3-174.3c-50-50-131-50-181 0L172 159c-12.6-10.1-31-9.3-42.6 2.4s-12.4 30-2.4 42.6L37.5 293.5c-50 50-50 131 0 181zM400 88a24 24 0 1 1 0 48 24 24 0 1 1 0-48zM304 248a24 24 0 1 1 0 48 24 24 0 1 1 0-48zM120 304a24 24 0 1 1 48 0 24 24 0 1 1 -48 0z";
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
    exports.faCucumber = exports.definition;
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

// src/_entries/faCucumber.js
var import_faCucumber = __toESM(require_faCucumber());
var export_faCucumber = import_faCucumber.definition;
export {
  export_faCucumber as faCucumber
};
