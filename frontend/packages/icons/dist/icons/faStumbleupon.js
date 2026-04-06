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

// src/node_modules/@fortawesome/free-brands-svg-icons/faStumbleupon.js
var require_faStumbleupon = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faStumbleupon.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "stumbleupon";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f1a4";
    var svgPathData = "M502.9 266l0 69.7c0 62.1-50.3 112.4-112.4 112.4-61.8 0-112.4-49.8-112.4-111.3l0-70.2 34.3 16 51.1-15.2 0 70.6c0 14.7 12 26.5 26.7 26.5S417 352.7 417 338l0-72 85.9 0zM278.2 207.8l34.3 16 51.1-15.2 0-35.6c0-60.5-51.1-109-112.1-109-60.8 0-112.1 48.2-112.1 108.2l0 162.4c0 14.9-12 26.7-26.7 26.7S86 349.5 86 334.6l0-68.6-86 0 0 69.7c0 62 50.3 112.3 112.4 112.3 61.6 0 112.4-49.5 112.4-110.8l0-160.3c0-14.7 12-26.7 26.7-26.7s26.7 12 26.7 26.7l0 30.9z";
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
    exports.faStumbleupon = exports.definition;
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

// src/_entries/faStumbleupon.js
var import_faStumbleupon = __toESM(require_faStumbleupon());
var export_faStumbleupon = import_faStumbleupon.definition;
export {
  export_faStumbleupon as faStumbleupon
};
