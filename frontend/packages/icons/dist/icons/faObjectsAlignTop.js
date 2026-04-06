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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faObjectsAlignTop.js
var require_faObjectsAlignTop = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faObjectsAlignTop.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "objects-align-top";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e3c0";
    var svgPathData = "M24 0L488 0c13.3 0 24 10.7 24 24s-10.7 24-24 24L24 48C10.7 48 0 37.3 0 24S10.7 0 24 0zM64 176c0-26.5 21.5-48 48-48l64 0c26.5 0 48 21.5 48 48l0 288c0 26.5-21.5 48-48 48l-64 0c-26.5 0-48-21.5-48-48l0-288zm224 0c0-26.5 21.5-48 48-48l64 0c26.5 0 48 21.5 48 48l0 160c0 26.5-21.5 48-48 48l-64 0c-26.5 0-48-21.5-48-48l0-160z";
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
    exports.faObjectsAlignTop = exports.definition;
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

// src/_entries/faObjectsAlignTop.js
var import_faObjectsAlignTop = __toESM(require_faObjectsAlignTop());
var export_faObjectsAlignTop = import_faObjectsAlignTop.definition;
export {
  export_faObjectsAlignTop as faObjectsAlignTop
};
