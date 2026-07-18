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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBorderBottomRight.js
var require_faBorderBottomRight = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBorderBottomRight.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "border-bottom-right";
    var width = 448;
    var height = 512;
    var aliases = ["border-style-alt"];
    var unicode = "f854";
    var svgPathData = "M448 64c0-17.7-14.3-32-32-32s-32 14.3-32 32l0 336c0 8.8-7.2 16-16 16L32 416c-17.7 0-32 14.3-32 32s14.3 32 32 32l336 0c44.2 0 80-35.8 80-80l0-336zM288 64a32 32 0 1 0 64 0 32 32 0 1 0 -64 0zM96 64a32 32 0 1 0 64 0A32 32 0 1 0 96 64zm96 0a32 32 0 1 0 64 0 32 32 0 1 0 -64 0zM0 64a32 32 0 1 0 64 0A32 32 0 1 0 0 64zM32 224a32 32 0 1 0 0 64 32 32 0 1 0 0-64zm0-32a32 32 0 1 0 0-64 32 32 0 1 0 0 64zm0 128a32 32 0 1 0 0 64 32 32 0 1 0 0-64z";
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
    exports.faBorderBottomRight = exports.definition;
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

// src/_entries/faBorderBottomRight.js
var import_faBorderBottomRight = __toESM(require_faBorderBottomRight());
var export_faBorderBottomRight = import_faBorderBottomRight.definition;
export {
  export_faBorderBottomRight as faBorderBottomRight
};
