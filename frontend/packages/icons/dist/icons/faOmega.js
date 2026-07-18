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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faOmega.js
var require_faOmega = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faOmega.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "omega";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f67a";
    var svgPathData = "M224 96C135.6 96 64 167.6 64 256c0 56.4 29.1 106 73.3 134.5c13.3 8.6 22.7 23.7 22.7 41.2c0 26.7-21.6 48.3-48.3 48.3L32 480c-17.7 0-32-14.3-32-32s14.3-32 32-32l35.2 0C25.8 375.4 0 318.7 0 256C0 132.3 100.3 32 224 32s224 100.3 224 224c0 62.7-25.8 119.4-67.2 160l35.2 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-79.7 0c-26.7 0-48.3-21.6-48.3-48.3c0-17.5 9.4-32.6 22.7-41.2C354.9 362 384 312.4 384 256c0-88.4-71.6-160-160-160z";
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
    exports.faOmega = exports.definition;
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

// src/_entries/faOmega.js
var import_faOmega = __toESM(require_faOmega());
var export_faOmega = import_faOmega.definition;
export {
  export_faOmega as faOmega
};
