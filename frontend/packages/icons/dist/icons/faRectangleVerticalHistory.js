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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faRectangleVerticalHistory.js
var require_faRectangleVerticalHistory = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faRectangleVerticalHistory.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "rectangle-vertical-history";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e237";
    var svgPathData = "M256 0c-35.3 0-64 28.7-64 64l0 384c0 35.3 28.7 64 64 64l256 0c35.3 0 64-28.7 64-64l0-384c0-35.3-28.7-64-64-64L256 0zM96 72l0 368c0 13.3 10.7 24 24 24s24-10.7 24-24l0-368c0-13.3-10.7-24-24-24s-24 10.7-24 24zM0 120L0 392c0 13.3 10.7 24 24 24s24-10.7 24-24l0-272c0-13.3-10.7-24-24-24S0 106.7 0 120z";
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
    exports.faRectangleVerticalHistory = exports.definition;
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

// src/_entries/faRectangleVerticalHistory.js
var import_faRectangleVerticalHistory = __toESM(require_faRectangleVerticalHistory());
var export_faRectangleVerticalHistory = import_faRectangleVerticalHistory.definition;
export {
  export_faRectangleVerticalHistory as faRectangleVerticalHistory
};
