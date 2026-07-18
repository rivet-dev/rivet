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

// src/node_modules/@fortawesome/free-solid-svg-icons/faVirgo.js
var require_faVirgo = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faVirgo.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "virgo";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e850";
    var svgPathData = "M368 32c61.9 0 112 50.1 112 112l0 26.8c14.5-6.9 30.8-10.8 48-10.8 61.9 0 112 50.1 112 112S589.9 384 528 384l-48 0 0 32c0 16.6 12.6 30.2 28.7 31.8l6.6 .3c16.1 1.6 28.7 15.3 28.7 31.8 0 17.7-14.3 32-32 32-53 0-96-43-96-96l0-32-16 0c-17.7 0-32-14.3-32-32s14.3-32 32-32l16 0 0-176c0-26.5-21.5-48-48-48s-48 21.5-48 48l0 304c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-304c0-26.5-21.5-48-48-48s-48 21.5-48 48l0 304c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-320c0-16.6-12.6-30.2-28.7-31.8l-6.6-.3C44.6 94.2 32 80.6 32 64 32 46.3 46.3 32 64 32 91 32 115.3 43.1 132.7 61 152.6 43 179 32 208 32 239.3 32 267.7 44.9 288 65.7 308.3 44.9 336.6 32 368 32zM528 224c-26.5 0-48 21.5-48 48l0 48 48 0c26.5 0 48-21.5 48-48s-21.5-48-48-48z";
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
    exports.faVirgo = exports.definition;
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

// src/_entries/faVirgo.js
var import_faVirgo = __toESM(require_faVirgo());
var export_faVirgo = import_faVirgo.definition;
export {
  export_faVirgo as faVirgo
};
