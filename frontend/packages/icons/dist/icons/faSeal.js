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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSeal.js
var require_faSeal = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSeal.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "seal";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e241";
    var svgPathData = "M289.9 18.4c-18.7-18.7-49.1-18.7-67.9 0L176.5 64 112 64c-26.5 0-48 21.5-48 48l0 64.5L18.4 222.1c-18.7 18.7-18.7 49.1 0 67.9L64 335.5 64 400c0 26.5 21.5 48 48 48l64.5 0 45.6 45.6c18.7 18.7 49.1 18.7 67.9 0L335.5 448l64.5 0c26.5 0 48-21.5 48-48l0-64.5 45.6-45.6c18.7-18.7 18.7-49.1 0-67.9L448 176.5l0-64.5c0-26.5-21.5-48-48-48l-64.5 0L289.9 18.4z";
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
    exports.faSeal = exports.definition;
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

// src/_entries/faSeal.js
var import_faSeal = __toESM(require_faSeal());
var export_faSeal = import_faSeal.definition;
export {
  export_faSeal as faSeal
};
