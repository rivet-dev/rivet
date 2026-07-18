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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faNotes.js
var require_faNotes = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faNotes.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "notes";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e202";
    var svgPathData = "M160 32c-35.3 0-64 28.7-64 64l0 256c0 35.3 28.7 64 64 64l179.2 0c4.3 0 8.6-.4 12.8-1.3l0-94.7c0-17.7 14.3-32 32-32l94.7 0c.9-4.2 1.3-8.5 1.3-12.8L480 96c0-35.3-28.7-64-64-64L160 32zM384 352l0 45.7 0 18.3 96-96-18.3 0L416 320l-32 0 0 32zM296 464l-160 0c-48.6 0-88-39.4-88-88l0-224c0-13.3-10.7-24-24-24s-24 10.7-24 24L0 376c0 75.1 60.9 136 136 136l160 0c13.3 0 24-10.7 24-24s-10.7-24-24-24z";
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
    exports.faNotes = exports.definition;
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

// src/_entries/faNotes.js
var import_faNotes = __toESM(require_faNotes());
var export_faNotes = import_faNotes.definition;
export {
  export_faNotes as faNotes
};
