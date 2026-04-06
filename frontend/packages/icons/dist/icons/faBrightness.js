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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBrightness.js
var require_faBrightness = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBrightness.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "brightness";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e0c9";
    var svgPathData = "M224 80l0-48c0-17.7 14.3-32 32-32s32 14.3 32 32l0 48c0 17.7-14.3 32-32 32s-32-14.3-32-32zM160 256a96 96 0 1 1 192 0 96 96 0 1 1 -192 0zM0 256c0-17.7 14.3-32 32-32l48 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-48 0c-17.7 0-32-14.3-32-32zm432-32l48 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-48 0c-17.7 0-32-14.3-32-32s14.3-32 32-32zM256 512c-17.7 0-32-14.3-32-32l0-48c0-17.7 14.3-32 32-32s32 14.3 32 32l0 48c0 17.7-14.3 32-32 32zM73.4 438.6c-12.5-12.5-12.5-32.8 0-45.3l32-32c12.5-12.5 32.8-12.5 45.3 0s12.5 32.8 0 45.3l-32 32c-12.5 12.5-32.8 12.5-45.3 0zm288-333.3l32-32c12.5-12.5 32.8-12.5 45.3 0s12.5 32.8 0 45.3l-32 32c-12.5 12.5-32.8 12.5-45.3 0s-12.5-32.8 0-45.3zm77.3 333.3c-12.5 12.5-32.8 12.5-45.3 0l-32-32c-12.5-12.5-12.5-32.8 0-45.3s32.8-12.5 45.3 0l32 32c12.5 12.5 12.5 32.8 0 45.3zm-333.3-288l-32-32c-12.5-12.5-12.5-32.8 0-45.3s32.8-12.5 45.3 0l32 32c12.5 12.5 12.5 32.8 0 45.3s-32.8 12.5-45.3 0z";
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
    exports.faBrightness = exports.definition;
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

// src/_entries/faBrightness.js
var import_faBrightness = __toESM(require_faBrightness());
var export_faBrightness = import_faBrightness.definition;
export {
  export_faBrightness as faBrightness
};
