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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faGarlic.js
var require_faGarlic = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faGarlic.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "garlic";
    var width = 512;
    var height = 512;
    var aliases = [129476];
    var unicode = "e40e";
    var svgPathData = "M83.4 169.1C42.3 200.5 0 232.7 0 288C0 392.6 85.5 411.7 116.7 415.2C85 390.5 64 352.6 64 304c0-27.8 4.3-48.7 11-66.5c5.9-15.6 13.6-28.6 20.7-40.5c.8-1.4 1.7-2.8 2.5-4.2c7.8-13.2 15.1-26.2 20.5-43.3c1.2-3.7 2.3-7.7 3.3-11.9c-11.7 10.9-25.1 21.1-38.7 31.5zM384 416s128 0 128-128c0-55.3-42.3-87.5-83.4-118.9C389.8 139.5 352 110.6 352 64l0-16c0-26.5-21.5-48-48-48L288 0l0 80c0 30.2 4.7 51.9 11.4 69c6.7 17.1 15.7 30.3 25.5 43.8c1.2 1.6 2.4 3.3 3.6 4.9c0 0 0 0 0 0s0 0 0 0c19 25.8 39.4 53.5 39.4 106.3c0 49.2-21.5 87.3-53.7 112l69.7 0zM256 80l0-80L208 0c-26.5 0-48 21.5-48 48l0 16 0 16c0 33.4-4.2 58.5-10.8 79.2c-6.6 20.7-15.3 36.2-23.5 49.9l-2.5 4.2c-7.2 12.1-13.5 22.6-18.3 35.5c-5.3 13.9-9 31-9 55.2c0 69.3 54.9 112 120 112s120-42.7 120-112c0-42.1-15-62.5-33.5-87.7c0 0 0 0 0 0c0 0 0 0 0-.1c-1.1-1.5-2.2-3.1-3.4-4.6c-10.2-13.9-21.2-29.8-29.5-50.9C261.3 139.5 256 113.8 256 80zM136 448l0 40c0 13.3 10.7 24 24 24s24-10.7 24-24l0-40-48 0zm64 0l0 40c0 13.3 10.7 24 24 24s24-10.7 24-24l0-40-48 0zm64 0l0 40c0 13.3 10.7 24 24 24s24-10.7 24-24l0-40-48 0zm64 0l0 40c0 13.3 10.7 24 24 24s24-10.7 24-24l0-40-48 0z";
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
    exports.faGarlic = exports.definition;
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

// src/_entries/faGarlic.js
var import_faGarlic = __toESM(require_faGarlic());
var export_faGarlic = import_faGarlic.definition;
export {
  export_faGarlic as faGarlic
};
