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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faChevronsDown.js
var require_faChevronsDown = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faChevronsDown.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "chevrons-down";
    var width = 512;
    var height = 512;
    var aliases = ["chevron-double-down"];
    var unicode = "f322";
    var svgPathData = "M278.6 470.6c-12.5 12.5-32.8 12.5-45.3 0l-192-192c-12.5-12.5-12.5-32.8 0-45.3s32.8-12.5 45.3 0L256 402.7 425.4 233.4c12.5-12.5 32.8-12.5 45.3 0s12.5 32.8 0 45.3l-192 192zm192-384l-192 192c-12.5 12.5-32.8 12.5-45.3 0l-192-192c-12.5-12.5-12.5-32.8 0-45.3s32.8-12.5 45.3 0L256 210.7 425.4 41.4c12.5-12.5 32.8-12.5 45.3 0s12.5 32.8 0 45.3z";
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
    exports.faChevronsDown = exports.definition;
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

// src/_entries/faChevronDoubleDown.js
var import_faChevronsDown = __toESM(require_faChevronsDown());
var export_faChevronDoubleDown = import_faChevronsDown.definition;
export {
  export_faChevronDoubleDown as faChevronDoubleDown
};
