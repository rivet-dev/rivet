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

// src/node_modules/@fortawesome/free-solid-svg-icons/faDongSign.js
var require_faDongSign = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faDongSign.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "dong-sign";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "e169";
    var svgPathData = "M288-16c-17.7 0-32 14.3-32 32l0 32-104 0c-13.3 0-24 10.7-24 24s10.7 24 24 24l104 0 0 72.2c-22.9-15.3-50.4-24.2-80-24.2-79.5 0-144 64.5-144 144S96.5 432 176 432c30 0 57.8-9.1 80.8-24.8 3.3 14.2 16 24.8 31.2 24.8 17.7 0 32-14.3 32-32l0-304 40 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-40 0 0-32c0-17.7-14.3-32-32-32zM96 288a80 80 0 1 1 160 0 80 80 0 1 1 -160 0zM24 464c-13.3 0-24 10.7-24 24s10.7 24 24 24l336 0c13.3 0 24-10.7 24-24s-10.7-24-24-24L24 464z";
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
    exports.faDongSign = exports.definition;
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

// src/_entries/faDongSign.js
var import_faDongSign = __toESM(require_faDongSign());
var export_faDongSign = import_faDongSign.definition;
export {
  export_faDongSign as faDongSign
};
