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

// src/node_modules/@fortawesome/free-solid-svg-icons/faSort.js
var require_faSort = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faSort.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "sort";
    var width = 384;
    var height = 512;
    var aliases = ["unsorted"];
    var unicode = "f0dc";
    var svgPathData = "M2.4 204.2c5 12 16.6 19.8 29.6 19.8l320 0c12.9 0 24.6-7.8 29.6-19.8s2.2-25.7-6.9-34.9l-160-160c-12.5-12.5-32.8-12.5-45.3 0l-160 160c-9.2 9.2-11.9 22.9-6.9 34.9zm0 103.5c-5 12-2.2 25.7 6.9 34.9l160 160c12.5 12.5 32.8 12.5 45.3 0l160-160c9.2-9.2 11.9-22.9 6.9-34.9S364.9 288 352 288L32 288c-12.9 0-24.6 7.8-29.6 19.8z";
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
    exports.faSort = exports.definition;
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

// src/_entries/faUnsorted.js
var import_faSort = __toESM(require_faSort());
var export_faUnsorted = import_faSort.definition;
export {
  export_faUnsorted as faUnsorted
};
