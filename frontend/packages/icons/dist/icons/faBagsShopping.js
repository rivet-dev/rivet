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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBagsShopping.js
var require_faBagsShopping = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBagsShopping.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bags-shopping";
    var width = 576;
    var height = 512;
    var aliases = [128717];
    var unicode = "f847";
    var svgPathData = "M176 96l0 64 96 0 0-64c0-26.5-21.5-48-48-48s-48 21.5-48 48zm-48 64l0-64c0-53 43-96 96-96s96 43 96 96l0 64 64 0c35.3 0 64 28.7 64 64l-208 0c-44.2 0-80 35.8-80 80l0 160c0 5.5 .6 10.8 1.6 16L64 480c-35.3 0-64-28.7-64-64L0 224c0-35.3 28.7-64 64-64l64 0zm128 96l256 0c35.3 0 64 28.7 64 64l0 128c0 35.3-28.7 64-64 64l-256 0c-35.3 0-64-28.7-64-64l0-128c0-35.3 28.7-64 64-64zm64 80c0-8.8-7.2-16-16-16s-16 7.2-16 16l0 16c0 53 43 96 96 96s96-43 96-96l0-16c0-8.8-7.2-16-16-16s-16 7.2-16 16l0 16c0 35.3-28.7 64-64 64s-64-28.7-64-64l0-16z";
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
    exports.faBagsShopping = exports.definition;
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

// src/_entries/faBagsShopping.js
var import_faBagsShopping = __toESM(require_faBagsShopping());
var export_faBagsShopping = import_faBagsShopping.definition;
export {
  export_faBagsShopping as faBagsShopping
};
