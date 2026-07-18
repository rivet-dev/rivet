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

// src/node_modules/@fortawesome/free-brands-svg-icons/faCcAmex.js
var require_faCcAmex = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faCcAmex.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "cc-amex";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f1f3";
    var svgPathData = "M0 432c0 26.5 21.5 48 48 48l480 0c26.5 0 48-21.5 48-48l0-1.1-61.7 0-31.9-35.1-31.9 35.1-203.7 0 0-163.8-65.8 0 81.7-184.7 78.6 0 28.1 63.2 0-63.2 97.2 0 16.9 47.6 17-47.6 75.5 0 0-2.4c0-26.5-21.5-48-48-48L48 32C21.5 32 0 53.5 0 80L0 432zm440.4-21.7l42.2-46.3 42 46.3 51.4 0-68-72.1 68-72.1-50.6 0-42 46.7-41.5-46.7-51.4 0 67.5 72.5-67.4 71.6 0-33.1-83 0 0-22.2 80.9 0 0-32.3-80.9 0 0-22.4 83 0 0-33.1-122 0 0 143.2 171.8 0zm96.3-72l39.3 41.9 0-83.3-39.3 41.4zm-36.3-92l36.9-100.6 0 100.6 38.7 0 0-143.3-60.2 0-32.2 89.3-31.9-89.3-61.2 0 0 143.1-63.2-143.1-51.2 0-62.4 143.3 43 0 11.9-28.7 65.9 0 12 28.7 82.7 0 0-100.3 36.8 100.3 34.4 0zM282 185.4l19.5-46.9 19.4 46.9-38.9 0z";
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
    exports.faCcAmex = exports.definition;
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

// src/_entries/faCcAmex.js
var import_faCcAmex = __toESM(require_faCcAmex());
var export_faCcAmex = import_faCcAmex.definition;
export {
  export_faCcAmex as faCcAmex
};
