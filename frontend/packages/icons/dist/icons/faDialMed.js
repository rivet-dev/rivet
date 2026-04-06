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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faDialMed.js
var require_faDialMed = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faDialMed.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "dial-med";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e15f";
    var svgPathData = "M288 0a32 32 0 1 1 0 64 32 32 0 1 1 0-64zM128 287.2C128 207 187 140.6 264 129l0 158.2c0 13.3 10.7 24 24 24s24-10.7 24-24L312 129c77 11.6 136 78 136 158.2c0 88.4-71.6 160-160 160s-160-71.6-160-160zm384 .8a32 32 0 1 1 64 0 32 32 0 1 1 -64 0zM32 256a32 32 0 1 1 0 64 32 32 0 1 1 0-64zM64 96a32 32 0 1 1 64 0A32 32 0 1 1 64 96zM480 64a32 32 0 1 1 0 64 32 32 0 1 1 0-64zM64 480a32 32 0 1 1 64 0 32 32 0 1 1 -64 0zm416-32a32 32 0 1 1 0 64 32 32 0 1 1 0-64z";
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
    exports.faDialMed = exports.definition;
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

// src/_entries/faDialMed.js
var import_faDialMed = __toESM(require_faDialMed());
var export_faDialMed = import_faDialMed.definition;
export {
  export_faDialMed as faDialMed
};
