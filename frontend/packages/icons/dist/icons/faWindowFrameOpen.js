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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faWindowFrameOpen.js
var require_faWindowFrameOpen = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faWindowFrameOpen.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "window-frame-open";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e050";
    var svgPathData = "M480 224l0-160c0-35.3-28.7-64-64-64L96 0C60.7 0 32 28.7 32 64l0 160 0 64 64 0 320 0 64 0 0-64zm-64 0l-136 0 0-160 136 0 0 160zm-184 0L96 224 96 64l136 0 0 160zM96 448l0-128-64 0 0 128c-17.7 0-32 14.3-32 32s14.3 32 32 32l32 0 384 0 32 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l0-128-64 0 0 128L96 448z";
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
    exports.faWindowFrameOpen = exports.definition;
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

// src/_entries/faWindowFrameOpen.js
var import_faWindowFrameOpen = __toESM(require_faWindowFrameOpen());
var export_faWindowFrameOpen = import_faWindowFrameOpen.definition;
export {
  export_faWindowFrameOpen as faWindowFrameOpen
};
