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

// src/node_modules/@fortawesome/free-solid-svg-icons/faArrowDownUpLock.js
var require_faArrowDownUpLock = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faArrowDownUpLock.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "arrow-down-up-lock";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e4b0";
    var svgPathData = "M262.6 422.6l-80 80c-12.5 12.5-32.8 12.5-45.3 0l-80-80c-12.5-12.5-12.5-32.8 0-45.3s32.8-12.5 45.3 0l25.4 25.4 0-114.7-64 0c-17.7 0-32-14.3-32-32s14.3-32 32-32l256 0 0-114.7-25.4 25.4c-12.5 12.5-32.8 12.5-45.3 0s-12.5-32.8 0-45.3l80-80c12.5-12.5 32.8-12.5 45.3 0l80 80c12.5 12.5 12.5 32.8 0 45.3s-32.8 12.5-45.3 0l-25.4-25.4 0 94.8c-25.3 20.3-42.8 50.1-47 83.9l-145 0 0 114.7 25.4-25.4c12.5-12.5 32.8-12.5 45.3 0s12.5 32.8 0 45.3zM192 32l0 144-64 0 0-144c0-17.7 14.3-32 32-32s32 14.3 32 32zM496 304.1c0-17.7-14.3-32-32-32s-32 14.3-32 32l0 47.9 64 0 0-47.9zM352 400c0-20.9 13.4-38.7 32-45.3l0-50.6c0-44.2 35.8-80 80-80s80 35.8 80 80l0 50.6c18.6 6.6 32 24.4 32 45.3l0 96c0 26.5-21.5 48-48 48l-128 0c-26.5 0-48-21.5-48-48l0-96z";
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
    exports.faArrowDownUpLock = exports.definition;
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

// src/_entries/faArrowDownUpLock.js
var import_faArrowDownUpLock = __toESM(require_faArrowDownUpLock());
var export_faArrowDownUpLock = import_faArrowDownUpLock.definition;
export {
  export_faArrowDownUpLock as faArrowDownUpLock
};
