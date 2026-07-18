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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPhoneRotary.js
var require_faPhoneRotary = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPhoneRotary.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "phone-rotary";
    var width = 512;
    var height = 512;
    var aliases = [9742];
    var unicode = "f8d3";
    var svgPathData = "M0 141.3c0-8.5 3.4-16.6 9.9-22.1C38.2 95.4 125.9 32 256 32s217.8 63.4 246.1 87.2c6.5 5.5 9.9 13.6 9.9 22.1l0 34.7c0 17.7-14.3 32-32 32l-42.3 0c-13.1 0-24.9-8-29.7-20.1l-19.4-48.6c-2.9-7.3-8.4-13.2-15.7-16c-18.6-7-59.5-19.3-116.8-19.3s-98.2 12.3-116.8 19.3c-7.3 2.8-12.8 8.7-15.7 16L104 187.9C99.2 200 87.4 208 74.3 208L32 208c-17.7 0-32-14.3-32-32l0-34.7zM32 432l0-31c0-11.1 2.9-22.1 8.4-31.8l101.1-177C153 172.3 174.2 160 197.1 160l117.7 0c23 0 44.2 12.3 55.6 32.2l101.1 177c5.5 9.7 8.4 20.6 8.4 31.8l0 31c0 26.5-21.5 48-48 48L80 480c-26.5 0-48-21.5-48-48zM328 312a72 72 0 1 0 -144 0 72 72 0 1 0 144 0z";
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
    exports.faPhoneRotary = exports.definition;
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

// src/_entries/faPhoneRotary.js
var import_faPhoneRotary = __toESM(require_faPhoneRotary());
var export_faPhoneRotary = import_faPhoneRotary.definition;
export {
  export_faPhoneRotary as faPhoneRotary
};
