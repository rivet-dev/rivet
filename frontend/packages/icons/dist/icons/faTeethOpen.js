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

// src/node_modules/@fortawesome/free-solid-svg-icons/faTeethOpen.js
var require_faTeethOpen = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faTeethOpen.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "teeth-open";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f62f";
    var svgPathData = "M64 32C28.7 32 0 60.7 0 96l0 80c0 35.3 28.7 64 64 64l384 0c35.3 0 64-28.7 64-64l0-80c0-35.3-28.7-64-64-64L64 32zm0 272c-35.3 0-64 28.7-64 64l0 48c0 35.3 28.7 64 64 64l384 0c35.3 0 64-28.7 64-64l0-48c0-35.3-28.7-64-64-64L64 304zm80-160c0-26.5 21.5-48 48-48s48 21.5 48 48l0 24c0 13.3-10.7 24-24 24l-48 0c-13.3 0-24-10.7-24-24l0-24zm128 0c0-26.5 21.5-48 48-48s48 21.5 48 48l0 24c0 13.3-10.7 24-24 24l-48 0c-13.3 0-24-10.7-24-24l0-24zM80 112c17.7 0 32 14.3 32 32l0 24c0 13.3-10.7 24-24 24l-16 0c-13.3 0-24-10.7-24-24l0-24c0-17.7 14.3-32 32-32zm320 32c0-17.7 14.3-32 32-32s32 14.3 32 32l0 24c0 13.3-10.7 24-24 24l-16 0c-13.3 0-24-10.7-24-24l0-24zM48 384l0-16c0-8.8 7.2-16 16-16l32 0c8.8 0 16 7.2 16 16l0 16c0 17.7-14.3 32-32 32s-32-14.3-32-32zm144 48c-26.5 0-48-21.5-48-48l0-16c0-8.8 7.2-16 16-16l64 0c8.8 0 16 7.2 16 16l0 16c0 26.5-21.5 48-48 48zm128 0c-26.5 0-48-21.5-48-48l0-16c0-8.8 7.2-16 16-16l64 0c8.8 0 16 7.2 16 16l0 16c0 26.5-21.5 48-48 48zm112-16c-17.7 0-32-14.3-32-32l0-16c0-8.8 7.2-16 16-16l32 0c8.8 0 16 7.2 16 16l0 16c0 17.7-14.3 32-32 32z";
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
    exports.faTeethOpen = exports.definition;
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

// src/_entries/faTeethOpen.js
var import_faTeethOpen = __toESM(require_faTeethOpen());
var export_faTeethOpen = import_faTeethOpen.definition;
export {
  export_faTeethOpen as faTeethOpen
};
