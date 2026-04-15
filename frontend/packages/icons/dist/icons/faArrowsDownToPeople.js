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

// src/node_modules/@fortawesome/free-solid-svg-icons/faArrowsDownToPeople.js
var require_faArrowsDownToPeople = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faArrowsDownToPeople.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "arrows-down-to-people";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e4b9";
    var svgPathData = "M113 153c-9.4 9.4-24.6 9.4-33.9 0L15 89C5.7 79.6 5.7 64.4 15 55S39.6 45.7 49 55L72 78.1 72-8c0-13.3 10.7-24 24-24s24 10.7 24 24l0 86.1 23-23c9.4-9.4 24.6-9.4 33.9 0s9.4 24.6 0 33.9l-64 64zm320 0c-9.4 9.4-24.6 9.4-33.9 0L335 89c-9.4-9.4-9.4-24.6 0-33.9s24.6-9.4 33.9 0l23 23 0-86.1c0-13.3 10.7-24 24-24s24 10.7 24 24l0 86.1 23-23c9.4-9.4 24.6-9.4 33.9 0s9.4 24.6 0 33.9l-64 64zM256 192a56 56 0 1 1 0 112 56 56 0 1 1 0-112zm0 160c53 0 96 43 96 96l0 32c0 17.7-14.3 32-32 32l-128 0c-17.7 0-32-14.3-32-32l0-32c0-53 43-96 96-96zM32 288a48 48 0 1 1 96 0 48 48 0 1 1 -96 0zm352 0a48 48 0 1 1 96 0 48 48 0 1 1 -96 0zM80 368c15.3 0 29.6 4.3 41.8 11.8-6.3 16.2-9.8 33.8-9.8 52.2l0 48c0 11.4 2.4 22.2 6.7 32l-87.8 0C13.8 512 0 498.2 0 481.1L0 448c0-44.2 35.8-80 80-80zM393.3 512c4.3-9.8 6.7-20.6 6.7-32l0-48c0-18.4-3.5-36-9.8-52.2 12.2-7.5 26.5-11.8 41.8-11.8 44.2 0 80 35.8 80 80l0 33.1c0 17-13.8 30.9-30.9 30.9l-87.8 0z";
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
    exports.faArrowsDownToPeople = exports.definition;
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

// src/_entries/faArrowsDownToPeople.js
var import_faArrowsDownToPeople = __toESM(require_faArrowsDownToPeople());
var export_faArrowsDownToPeople = import_faArrowsDownToPeople.definition;
export {
  export_faArrowsDownToPeople as faArrowsDownToPeople
};
