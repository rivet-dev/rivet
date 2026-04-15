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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCrosshairsSimple.js
var require_faCrosshairsSimple = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCrosshairsSimple.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "crosshairs-simple";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e59f";
    var svgPathData = "M445.3 224L384 224c-17.7 0-32 14.3-32 32s14.3 32 32 32l61.3 0C431.9 368.4 368.4 431.9 288 445.3l0-61.3c0-17.7-14.3-32-32-32s-32 14.3-32 32l0 61.3C143.6 431.9 80.1 368.4 66.7 288l61.3 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-61.3 0C80.1 143.6 143.6 80.1 224 66.7l0 61.3c0 17.7 14.3 32 32 32s32-14.3 32-32l0-61.3C368.4 80.1 431.9 143.6 445.3 224zM0 256a256 256 0 1 0 512 0A256 256 0 1 0 0 256z";
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
    exports.faCrosshairsSimple = exports.definition;
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

// src/_entries/faCrosshairsSimple.js
var import_faCrosshairsSimple = __toESM(require_faCrosshairsSimple());
var export_faCrosshairsSimple = import_faCrosshairsSimple.definition;
export {
  export_faCrosshairsSimple as faCrosshairsSimple
};
