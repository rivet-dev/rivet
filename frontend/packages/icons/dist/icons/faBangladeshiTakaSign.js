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

// src/node_modules/@fortawesome/free-solid-svg-icons/faBangladeshiTakaSign.js
var require_faBangladeshiTakaSign = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faBangladeshiTakaSign.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bangladeshi-taka-sign";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "e2e6";
    var svgPathData = "M36 32.3C18.4 30.1 2.4 42.5 .3 60S10.5 93.6 28 95.8l7.9 1c16 2 28 15.6 28 31.8l0 31.5-40 0c-13.3 0-24 10.7-24 24s10.7 24 24 24l40 0 0 176c0 53 43 96 96 96l32 0c106 0 192-86 192-192l0-32c0-53-43-96-96-96l-16 0c-17.7 0-32 14.3-32 32s14.3 32 32 32l16 0c17.7 0 32 14.3 32 32l0 32c0 70.7-57.3 128-128 128l-32 0c-17.7 0-32-14.3-32-32l0-176 40 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-40 0 0-31.5C128 80.1 92 39.2 43.9 33.2l-7.9-1z";
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
    exports.faBangladeshiTakaSign = exports.definition;
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

// src/_entries/faBangladeshiTakaSign.js
var import_faBangladeshiTakaSign = __toESM(require_faBangladeshiTakaSign());
var export_faBangladeshiTakaSign = import_faBangladeshiTakaSign.definition;
export {
  export_faBangladeshiTakaSign as faBangladeshiTakaSign
};
