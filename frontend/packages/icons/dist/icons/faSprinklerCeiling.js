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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSprinklerCeiling.js
var require_faSprinklerCeiling = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSprinklerCeiling.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "sprinkler-ceiling";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "e44c";
    var svgPathData = "M0 0L0 96c0 15.3 10.8 28.4 25.7 31.4L128 147.8l0-65.3L64 69.8 64 0 0 0zM256 82.6l0 65.3 102.3-20.5c15-3 25.7-16.1 25.7-31.4l0-96L320 0l0 69.8L256 82.6zM160 344a24 24 0 1 0 -48 0 24 24 0 1 0 48 0zm-48 64a24 24 0 1 0 -48 0 24 24 0 1 0 48 0zM24 496a24 24 0 1 0 0-48 24 24 0 1 0 0 48zM272 344a24 24 0 1 0 -48 0 24 24 0 1 0 48 0zm-80 88a24 24 0 1 0 0-48 24 24 0 1 0 0 48zm-32 40a24 24 0 1 0 -48 0 24 24 0 1 0 48 0zm136-40a24 24 0 1 0 0-48 24 24 0 1 0 0 48zm-24 40a24 24 0 1 0 -48 0 24 24 0 1 0 48 0zm88 24a24 24 0 1 0 0-48 24 24 0 1 0 0 48zM224 64c0-17.7-14.3-32-32-32s-32 14.3-32 32l0 128-64 0c-17.7 0-32 14.3-32 32s14.3 32 32 32l96 0 96 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-64 0 0-128z";
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
    exports.faSprinklerCeiling = exports.definition;
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

// src/_entries/faSprinklerCeiling.js
var import_faSprinklerCeiling = __toESM(require_faSprinklerCeiling());
var export_faSprinklerCeiling = import_faSprinklerCeiling.definition;
export {
  export_faSprinklerCeiling as faSprinklerCeiling
};
