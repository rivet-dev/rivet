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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faNfc.js
var require_faNfc = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faNfc.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "nfc";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e1f7";
    var svgPathData = "M0 96C0 60.7 28.7 32 64 32l320 0c35.3 0 64 28.7 64 64l0 320c0 35.3-28.7 64-64 64L64 480c-35.3 0-64-28.7-64-64L0 96zm64 48l0 224c0 26.5 21.5 48 48 48l224 0c26.5 0 48-21.5 48-48l0-224c0-26.5-21.5-48-48-48l-80 0c-26.5 0-48 21.5-48 48l0 66.7c-18.6 6.6-32 24.4-32 45.3c0 26.5 21.5 48 48 48s48-21.5 48-48c0-20.9-13.4-38.7-32-45.3l0-66.7c0-8.8 7.2-16 16-16l80 0c8.8 0 16 7.2 16 16l0 224c0 8.8-7.2 16-16 16l-224 0c-8.8 0-16-7.2-16-16l0-224c0-8.8 7.2-16 16-16l32 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-32 0c-26.5 0-48 21.5-48 48z";
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
    exports.faNfc = exports.definition;
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

// src/_entries/faNfc.js
var import_faNfc = __toESM(require_faNfc());
var export_faNfc = import_faNfc.definition;
export {
  export_faNfc as faNfc
};
