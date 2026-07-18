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

// src/node_modules/@fortawesome/free-solid-svg-icons/faMemory.js
var require_faMemory = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faMemory.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "memory";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f538";
    var svgPathData = "M64 64C28.7 64 0 92.7 0 128l0 7.4C0 142.2 4.4 148 10.1 151.7 23.3 160.3 32 175.1 32 192s-8.7 31.7-21.9 40.3C4.4 236 0 241.8 0 248.6l0 55.4 512 0 0-55.4c0-6.8-4.4-12.6-10.1-16.3-13.2-8.6-21.9-23.4-21.9-40.3s8.7-31.7 21.9-40.3c5.7-3.7 10.1-9.5 10.1-16.3l0-7.4c0-35.3-28.7-64-64-64L64 64zM512 416l0-64-512 0 0 64c0 17.7 14.3 32 32 32l64 0 0-24c0-13.3 10.7-24 24-24s24 10.7 24 24l0 24 88 0 0-24c0-13.3 10.7-24 24-24s24 10.7 24 24l0 24 88 0 0-24c0-13.3 10.7-24 24-24s24 10.7 24 24l0 24 64 0c17.7 0 32-14.3 32-32zM160 160l0 64c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-64c0-17.7 14.3-32 32-32s32 14.3 32 32zm128 0l0 64c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-64c0-17.7 14.3-32 32-32s32 14.3 32 32zm128 0l0 64c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-64c0-17.7 14.3-32 32-32s32 14.3 32 32z";
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
    exports.faMemory = exports.definition;
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

// src/_entries/faMemory.js
var import_faMemory = __toESM(require_faMemory());
var export_faMemory = import_faMemory.definition;
export {
  export_faMemory as faMemory
};
