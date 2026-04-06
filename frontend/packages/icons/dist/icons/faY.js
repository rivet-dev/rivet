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

// src/node_modules/@fortawesome/free-solid-svg-icons/faY.js
var require_faY = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faY.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "y";
    var width = 384;
    var height = 512;
    var aliases = [121];
    var unicode = "59";
    var svgPathData = "M58 45.4C47.8 31 27.8 27.7 13.4 38S-4.3 68.2 6 82.6L160 298.3 160 448c0 17.7 14.3 32 32 32s32-14.3 32-32l0-149.7 154-215.7c10.3-14.4 6.9-34.4-7.4-44.6S336.2 31 326 45.4L192 232.9 58 45.4z";
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
    exports.faY = exports.definition;
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

// src/_entries/faY.js
var import_faY = __toESM(require_faY());
var export_faY = import_faY.definition;
export {
  export_faY as faY
};
