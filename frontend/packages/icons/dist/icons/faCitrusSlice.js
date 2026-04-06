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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCitrusSlice.js
var require_faCitrusSlice = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCitrusSlice.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "citrus-slice";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e2f5";
    var svgPathData = "M41 391c-9.4-9.4-24.6-9.4-33.9 0s-9.4 24.6 0 33.9c115.6 115.6 302.6 116 418.3 .3S540.5 122.6 425 7c-9.4-9.4-24.6-9.4-33.9 0s-9.4 24.6 0 33.9c96.9 96.9 97.1 253.6 .3 350.4S137.9 487.9 41 391zm116.3 32.9c9.6 2.7 18.8-4.9 18.8-14.9l0-144.5c0-6.2-4.3-11.5-10-14c-5.9-2.6-13-2.1-17.6 2.5L59.3 342.2c-6.2 6.2-6.2 16.4 0 22.6l4 4c27.1 27.1 59.6 45.5 94 55.2zM250.5 166c2.5 5.7 7.8 10 14 10L409 176c10 0 17.6-9.1 14.9-18.8C414.2 123 396 90.6 369.1 63.6l-4.3-4.3c-6.2-6.2-16.4-6.2-22.6 0L253 148.4c-4.6 4.6-5.2 11.7-2.5 17.6zm.6 50.5c-2 4.9-1.4 10.6 2.4 14.3l114 114c6.6 6.6 17.4 6.2 22.9-1.3c26.2-35.8 40.1-77.8 41.5-120.1c.3-8.6-6.8-15.4-15.4-15.4L263 208c-5.3 0-9.8 3.7-11.8 8.5zm-34.6 34.6c-4.9 2-8.5 6.5-8.5 11.8l0 153.5c0 8.6 6.8 15.7 15.4 15.4c42.4-1.4 84.4-15.3 120.1-41.5c7.5-5.5 7.9-16.3 1.3-22.9l-114-114c-3.7-3.7-9.4-4.4-14.3-2.4z";
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
    exports.faCitrusSlice = exports.definition;
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

// src/_entries/faCitrusSlice.js
var import_faCitrusSlice = __toESM(require_faCitrusSlice());
var export_faCitrusSlice = import_faCitrusSlice.definition;
export {
  export_faCitrusSlice as faCitrusSlice
};
