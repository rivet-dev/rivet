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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPanFrying.js
var require_faPanFrying = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPanFrying.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "pan-frying";
    var width = 576;
    var height = 512;
    var aliases = [127859];
    var unicode = "e42c";
    var svgPathData = "M448 224c0 33.4-7.3 65.1-20.4 93.6l123.7 79.9C566.7 409.2 576 427.1 576 448c0 35.3-28.7 64-64 64c-15.2 0-28.3-4.6-39.3-13.5L364.8 398.3C326.3 429.4 277.3 448 224 448C100.3 448 0 347.7 0 224S100.3 0 224 0S448 100.3 448 224zM319.9 154c-21.2-5.4-50.6-22.6-65.8-38.4c-24.5-25.5-65-26.3-90.5-1.8c-11.6 11.2-18.1 25.6-19.4 40.5c-2.2 24.7-9.2 56.2-21.5 77.6c-22.1 38.3-9 87.2 29.3 109.3s87.2 9 109.3-29.3c9.4-16.3 33-30 51.6-32.6c10.2-1.4 20.3-5.4 29.2-12c28.4-21.1 34.3-61.1 13.3-89.5c-9.1-12.3-21.8-20.4-35.5-23.9zM528 448a16 16 0 1 0 -32 0 16 16 0 1 0 32 0zM224 176a48 48 0 1 1 0 96 48 48 0 1 1 0-96z";
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
    exports.faPanFrying = exports.definition;
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

// src/_entries/faPanFrying.js
var import_faPanFrying = __toESM(require_faPanFrying());
var export_faPanFrying = import_faPanFrying.definition;
export {
  export_faPanFrying as faPanFrying
};
