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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTrees.js
var require_faTrees = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTrees.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "trees";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f724";
    var svgPathData = "M70.2 121.4L178.5 5.8C182 2.1 186.9 0 192 0s10 2.1 13.5 5.8L313.8 121.4c4 4.3 6.2 9.9 6.2 15.7c0 12.7-10.3 22.9-22.9 22.9L272 160l74.6 89.5c3.5 4.2 5.4 9.5 5.4 15c0 13-10.5 23.5-23.5 23.5L304 288l74.6 89.5c3.5 4.2 5.4 9.5 5.4 15c0 13-10.5 23.5-23.5 23.5L224 416l0 64c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-64L23.5 416C10.5 416 0 405.5 0 392.5c0-5.5 1.9-10.8 5.4-15L80 288l-24.5 0c-13 0-23.5-10.5-23.5-23.5c0-5.5 1.9-10.8 5.4-15L112 160l-25.1 0C74.3 160 64 149.7 64 137.1c0-5.8 2.2-11.4 6.2-15.7zm271.4-16.4L434.5 5.8C438 2.1 442.9 0 448 0s10 2.1 13.5 5.8L569.8 121.4c4 4.3 6.2 9.9 6.2 15.7c0 12.7-10.3 22.9-22.9 22.9L528 160l74.6 89.5c3.5 4.2 5.4 9.5 5.4 15c0 13-10.5 23.5-23.5 23.5L560 288l74.6 89.5c3.5 4.2 5.4 9.5 5.4 15c0 13-10.5 23.5-23.5 23.5L480 416l0 64c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-64 0-23.4c0 0 0-.1 0-.1s0-.1 0-.1l0-.4s0 0 0 0c-.1-12.8-4.7-25.2-12.9-35l-40.6-48.7c13-10.2 21.4-26 21.4-43.8c0-13-4.6-25.6-12.9-35.5l-40.4-48.5c12.9-10 21.3-25.8 21.3-43.4c0-11.6-3.7-22.8-10.4-32.1z";
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
    exports.faTrees = exports.definition;
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

// src/_entries/faTrees.js
var import_faTrees = __toESM(require_faTrees());
var export_faTrees = import_faTrees.definition;
export {
  export_faTrees as faTrees
};
