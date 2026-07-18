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

// src/node_modules/@fortawesome/free-solid-svg-icons/faAustralSign.js
var require_faAustralSign = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faAustralSign.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "austral-sign";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e0a9";
    var svgPathData = "M266.5 240l-85 0 42.5-106.4 42.5 106.4zm68.9 0L262.9 58.3C256.5 42.4 241.1 32 224 32s-32.5 10.4-38.9 26.3L112.6 240 32 240c-13.3 0-24 10.7-24 24s10.7 24 24 24l61.4 0-19.2 48-42.3 0c-13.3 0-24 10.7-24 24s10.7 24 24 24l23.1 0-20.8 52.1c-6.6 16.4 1.4 35 17.9 41.6s35-1.4 41.6-17.9l30.3-75.9 200 0 30.3 75.9c6.6 16.4 25.2 24.4 41.6 17.9s24.4-25.2 17.9-41.6L392.9 384 416 384c13.3 0 24-10.7 24-24s-10.7-24-24-24l-42.3 0-19.2-48 61.4 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-80.6 0zm-49.7 48l19.2 48-161.6 0 19.2-48 123.3 0z";
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
    exports.faAustralSign = exports.definition;
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

// src/_entries/faAustralSign.js
var import_faAustralSign = __toESM(require_faAustralSign());
var export_faAustralSign = import_faAustralSign.definition;
export {
  export_faAustralSign as faAustralSign
};
