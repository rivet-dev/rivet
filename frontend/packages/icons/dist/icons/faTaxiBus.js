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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTaxiBus.js
var require_faTaxiBus = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTaxiBus.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "taxi-bus";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e298";
    var svgPathData = "M48 416l-16 0c-17.7 0-32-14.3-32-32L0 74.5c0-16.2 9.2-31 23.7-38.3C71.2 12.4 123.7 0 176.9 0l30.3 0c53.2 0 105.6 12.4 153.2 36.2C374.8 43.4 384 58.2 384 74.5L384 96c-21.9 0-40.4 14.7-46.2 34.8c-6.1 1.4-12.1 3.3-17.8 5.6l0-16.4c0-13.3-10.7-24-24-24l-88 0 0 128 41.9 0-16.3 48.9C208.5 290.2 192 319.2 192 352l0 64-64 0 0 32c0 17.7-14.3 32-32 32l-16 0c-17.7 0-32-14.3-32-32l0-32zM64 120l0 80c0 13.3 10.7 24 24 24l88 0 0-128L88 96c-13.3 0-24 10.7-24 24zm56 200a24 24 0 1 0 -48 0 24 24 0 1 0 48 0zM368 152c0-13.3 10.7-24 24-24l80 0c13.3 0 24 10.7 24 24l0 8 5.4 0c34.4 0 65 22 75.9 54.7l26.6 79.7C625.2 304.7 640 326.6 640 352l0 96 0 32c0 17.7-14.3 32-32 32l-16 0c-17.7 0-32-14.3-32-32l0-32-256 0 0 32c0 17.7-14.3 32-32 32l-16 0c-17.7 0-32-14.3-32-32l0-32s0 0 0 0l0-96c0-25.4 14.8-47.3 36.1-57.6l26.6-79.7C297.6 182 328.2 160 362.6 160l5.4 0 0-8zm-20.6 82.9L329.7 288l204.5 0-17.7-53.1c-2.2-6.5-8.3-10.9-15.2-10.9l-138.8 0c-6.9 0-13 4.4-15.2 10.9zM328 368a24 24 0 1 0 -48 0 24 24 0 1 0 48 0zm232 24a24 24 0 1 0 0-48 24 24 0 1 0 0 48z";
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
    exports.faTaxiBus = exports.definition;
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

// src/_entries/faTaxiBus.js
var import_faTaxiBus = __toESM(require_faTaxiBus());
var export_faTaxiBus = import_faTaxiBus.definition;
export {
  export_faTaxiBus as faTaxiBus
};
