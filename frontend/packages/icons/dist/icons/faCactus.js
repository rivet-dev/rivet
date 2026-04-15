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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCactus.js
var require_faCactus = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCactus.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "cactus";
    var width = 512;
    var height = 512;
    var aliases = [127797];
    var unicode = "f8a7";
    var svgPathData = "M160 96c0-53 43-96 96-96s96 43 96 96l0 256 48 0c8.8 0 16-7.2 16-16l0-64c0-26.5 21.5-48 48-48s48 21.5 48 48l0 64c0 61.9-50.1 112-112 112l-48 0 0 16c0 26.5-21.5 48-48 48l-96 0c-26.5 0-48-21.5-48-48l0-144-48 0C50.1 320 0 269.9 0 208l0-64c0-26.5 21.5-48 48-48s48 21.5 48 48l0 64c0 8.8 7.2 16 16 16l48 0 0-128zm80 64a16 16 0 1 0 -32 0 16 16 0 1 0 32 0zm48 240a16 16 0 1 0 0-32 16 16 0 1 0 0 32z";
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
    exports.faCactus = exports.definition;
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

// src/_entries/faCactus.js
var import_faCactus = __toESM(require_faCactus());
var export_faCactus = import_faCactus.definition;
export {
  export_faCactus as faCactus
};
