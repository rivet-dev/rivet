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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTruckLadder.js
var require_faTruckLadder = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTruckLadder.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "truck-ladder";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e657";
    var svgPathData = "M0 24C0 37.3 10.7 48 24 48l8 0 0 96-8 0c-13.3 0-24 10.7-24 24s10.7 24 24 24l304 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-8 0 0-96 8 0c13.3 0 24-10.7 24-24s-10.7-24-24-24L24 0C10.7 0 0 10.7 0 24zM272 144l-32 0 0-96 32 0 0 96zM160 48l32 0 0 96-32 0 0-96zm-48 96l-32 0 0-96 32 0 0 96zM416 96c-17.7 0-32 14.3-32 32l0 96L48 224c-26.5 0-48 21.5-48 48l0 96c0 26.5 21.5 48 48 48l16 0c0 53 43 96 96 96s96-43 96-96l128 0c0 53 43 96 96 96s96-43 96-96l16 0c26.5 0 48-21.5 48-48l0-80 0-32 0-18.7c0-17-6.7-33.3-18.7-45.3L544 114.7c-12-12-28.3-18.7-45.3-18.7L416 96zM576 256l-128 0 0-32 0-64 50.7 0L576 237.3l0 18.7zM112 416a48 48 0 1 1 96 0 48 48 0 1 1 -96 0zm368-48a48 48 0 1 1 0 96 48 48 0 1 1 0-96z";
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
    exports.faTruckLadder = exports.definition;
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

// src/_entries/faTruckLadder.js
var import_faTruckLadder = __toESM(require_faTruckLadder());
var export_faTruckLadder = import_faTruckLadder.definition;
export {
  export_faTruckLadder as faTruckLadder
};
