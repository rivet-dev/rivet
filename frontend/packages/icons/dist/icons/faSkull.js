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

// src/node_modules/@fortawesome/free-solid-svg-icons/faSkull.js
var require_faSkull = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faSkull.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "skull";
    var width = 512;
    var height = 512;
    var aliases = [128128];
    var unicode = "f54c";
    var svgPathData = "M416 427.4c58.5-44 96-111.6 96-187.4 0-132.5-114.6-240-256-240S0 107.5 0 240c0 75.8 37.5 143.4 96 187.4L96 464c0 26.5 21.5 48 48 48l32 0 0-40c0-13.3 10.7-24 24-24s24 10.7 24 24l0 40 64 0 0-40c0-13.3 10.7-24 24-24s24 10.7 24 24l0 40 32 0c26.5 0 48-21.5 48-48l0-36.6zM96 256a64 64 0 1 1 128 0 64 64 0 1 1 -128 0zm256-64a64 64 0 1 1 0 128 64 64 0 1 1 0-128z";
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
    exports.faSkull = exports.definition;
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

// src/_entries/faSkull.js
var import_faSkull = __toESM(require_faSkull());
var export_faSkull = import_faSkull.definition;
export {
  export_faSkull as faSkull
};
