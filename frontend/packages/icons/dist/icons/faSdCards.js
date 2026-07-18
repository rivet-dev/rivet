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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSdCards.js
var require_faSdCards = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSdCards.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "sd-cards";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e240";
    var svgPathData = "M384 0L237.3 0C220.3 0 204 6.7 192 18.7L114.7 96c-12 12-18.7 28.3-18.7 45.3L96 352c0 35.3 28.7 64 64 64l224 0c35.3 0 64-28.7 64-64l0-288c0-35.3-28.7-64-64-64zM280 64c13.3 0 24 10.7 24 24l0 48c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-48c0-13.3 10.7-24 24-24zm80 0c13.3 0 24 10.7 24 24l0 48c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-48c0-13.3 10.7-24 24-24zM48 120c0-13.3-10.7-24-24-24S0 106.7 0 120L0 376c0 75.1 60.9 136 136 136l224 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-224 0c-48.6 0-88-39.4-88-88l0-256z";
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
    exports.faSdCards = exports.definition;
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

// src/_entries/faSdCards.js
var import_faSdCards = __toESM(require_faSdCards());
var export_faSdCards = import_faSdCards.definition;
export {
  export_faSdCards as faSdCards
};
