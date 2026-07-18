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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTruckTow.js
var require_faTruckTow = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTruckTow.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "truck-tow";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e2b8";
    var svgPathData = "M96 0l49.6 0c9.1 0 17.8 3.9 23.9 10.7L384 252l0-124c0-17.7 14.3-32 32-32l82.7 0c17 0 33.3 6.7 45.3 18.7L621.3 192c12 12 18.7 28.3 18.7 45.3l0 18.7 0 32 0 88c0 22.1-17.9 40-40 40l-24 0c0 53-43 96-96 96s-96-43-96-96l-128 0c0 53-43 96-96 96s-96-43-96-96l-16 0c-26.5 0-48-21.5-48-48l0-64c0-26.5 21.5-48 48-48l144 0L96 64l0 112c0 26.5-21.5 48-48 48s-48-21.5-48-48l0-16c0-8.8 7.2-16 16-16s16 7.2 16 16l0 16c0 8.8 7.2 16 16 16s16-7.2 16-16L64 32C64 14.3 78.3 0 96 0zM576 256l0-18.7L498.7 160 448 160l0 96 128 0zM208 416a48 48 0 1 0 -96 0 48 48 0 1 0 96 0zm272 48a48 48 0 1 0 0-96 48 48 0 1 0 0 96z";
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
    exports.faTruckTow = exports.definition;
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

// src/_entries/faTruckTow.js
var import_faTruckTow = __toESM(require_faTruckTow());
var export_faTruckTow = import_faTruckTow.definition;
export {
  export_faTruckTow as faTruckTow
};
