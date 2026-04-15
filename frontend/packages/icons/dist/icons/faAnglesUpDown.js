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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faAnglesUpDown.js
var require_faAnglesUpDown = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faAnglesUpDown.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "angles-up-down";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e60d";
    var svgPathData = "M244 7c-11.7-9.3-28.3-9.3-40 0L44 135c-13.8 11-16 31.2-5 45s31.2 16 45 5L224 73 364 185c13.8 11 33.9 8.8 45-5s8.8-33.9-5-45L244 7zM404 377c13.8-11 16-31.2 5-45s-31.2-16-45-5L224 439 84 327c-13.8-11-33.9-8.8-45 5s-8.8 33.9 5 45L204 505c11.7 9.3 28.3 9.3 40 0L404 377z";
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
    exports.faAnglesUpDown = exports.definition;
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

// src/_entries/faAnglesUpDown.js
var import_faAnglesUpDown = __toESM(require_faAnglesUpDown());
var export_faAnglesUpDown = import_faAnglesUpDown.definition;
export {
  export_faAnglesUpDown as faAnglesUpDown
};
