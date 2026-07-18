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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faMobileSignal.js
var require_faMobileSignal = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faMobileSignal.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "mobile-signal";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e1ef";
    var svgPathData = "M64 64l224 0 0 251.7c18.3-21.2 39.9-39.5 64-54L352 64c0-35.3-28.7-64-64-64L64 0C28.7 0 0 28.7 0 64L0 448c0 35.3 28.7 64 64 64l160 0 0-24c0-13.6 1-27 3-40L64 448 64 64zm80 320c-8.8 0-16 7.2-16 16s7.2 16 16 16l64 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-64 0zM488 256c-128.1 0-232 103.9-232 232c0 13.3 10.7 24 24 24s24-10.7 24-24c0-101.6 82.4-184 184-184c13.3 0 24-10.7 24-24s-10.7-24-24-24zm-8 256a32 32 0 1 0 0-64 32 32 0 1 0 0 64zm32-136c0-13.3-10.7-24-24-24c-75.1 0-136 60.9-136 136c0 13.3 10.7 24 24 24s24-10.7 24-24c0-48.6 39.4-88 88-88c13.3 0 24-10.7 24-24z";
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
    exports.faMobileSignal = exports.definition;
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

// src/_entries/faMobileSignal.js
var import_faMobileSignal = __toESM(require_faMobileSignal());
var export_faMobileSignal = import_faMobileSignal.definition;
export {
  export_faMobileSignal as faMobileSignal
};
