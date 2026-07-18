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

// src/node_modules/@fortawesome/free-solid-svg-icons/faMarsStrokeUp.js
var require_faMarsStrokeUp = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faMarsStrokeUp.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "mars-stroke-up";
    var width = 384;
    var height = 512;
    var aliases = [9896, "mars-stroke-v"];
    var unicode = "f22a";
    var svgPathData = "M192 256a112 112 0 1 0 0 224 112 112 0 1 0 0-224zM16 368c0-86.3 62.1-158.1 144.1-173.1-.1-.9-.1-1.9-.1-2.9l0-16-32 0c-17.7 0-32-14.3-32-32s14.3-32 32-32l32 0 0-61.4-28 22.4c-13.8 11-33.9 8.8-45-5s-8.8-33.9 5-45l80-64c11.7-9.3 28.3-9.3 40 0l80 64c13.8 11 16 31.2 5 45s-31.2 16-45 5l-28-22.4 0 61.4 32 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-32 0 0 16c0 1 0 1.9-.1 2.9 82 15 144.1 86.8 144.1 173.1 0 97.2-78.8 176-176 176S16 465.2 16 368z";
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
    exports.faMarsStrokeUp = exports.definition;
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

// src/_entries/faMarsStrokeUp.js
var import_faMarsStrokeUp = __toESM(require_faMarsStrokeUp());
var export_faMarsStrokeUp = import_faMarsStrokeUp.definition;
export {
  export_faMarsStrokeUp as faMarsStrokeUp
};
