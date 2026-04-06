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

// src/node_modules/@fortawesome/free-solid-svg-icons/faHeadSideMask.js
var require_faHeadSideMask = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faHeadSideMask.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "head-side-mask";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e063";
    var svgPathData = "M445.9 193c-.1-.4-.2-.9-.2-1.3-15.7-108.4-108.9-191.7-221.7-191.7-85 0-159 47.4-196.9 117.2L283.3 288 456 288 442.9 336 336 336c-8.8 0-16 7.2-16 16s7.2 16 16 16l98.2 0-8.7 32-89.5 0c-8.8 0-16 7.2-16 16s7.2 16 16 16l80.6 0c-6.7 19.1-24.8 32-45.3 32L288 464c-17.7 0-32-14.3-32-32l0-104.5-247.5-165C3 182.1 0 202.7 0 224 0 278 19.1 327.5 50.9 366.2 58.9 375.9 64 387.8 64 400.4L64 464c0 26.5 21.5 48 48 48l127.3 0 .7 0 131.3 0c44.9 0 83.8-31.1 93.6-74.9l33.9-150.3c3.3-14.4-.3-29.5-9.7-41L445.9 193zM288 192a32 32 0 1 1 64 0 32 32 0 1 1 -64 0z";
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
    exports.faHeadSideMask = exports.definition;
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

// src/_entries/faHeadSideMask.js
var import_faHeadSideMask = __toESM(require_faHeadSideMask());
var export_faHeadSideMask = import_faHeadSideMask.definition;
export {
  export_faHeadSideMask as faHeadSideMask
};
