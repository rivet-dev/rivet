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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faMelonSlice.js
var require_faMelonSlice = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faMelonSlice.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "melon-slice";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e311";
    var svgPathData = "M41 391c-9.4-9.4-24.6-9.4-33.9 0s-9.4 24.6 0 33.9c115.6 115.6 302.6 116 418.3 .3S540.5 122.6 425 7c-9.4-9.4-24.6-9.4-33.9 0s-9.4 24.6 0 33.9c96.9 96.9 97.1 253.6 .3 350.4S137.9 487.9 41 391zM320 128c0 106-86 192-192 192c-11.4 0-22.5-1-33.3-2.9c-5.4-.9-11 .6-14.8 4.5L59.3 342.2c-6.2 6.2-6.2 16.4 0 22.6l4 4c84.4 84.4 221.1 84.4 305.5 0c84.2-84.2 84.4-220.7 .3-305.1l-4.3-4.3c-6.2-6.2-16.4-6.2-22.6 0L321.6 79.8c-3.9 3.9-5.4 9.4-4.5 14.8c1.9 10.8 2.9 22 2.9 33.3z";
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
    exports.faMelonSlice = exports.definition;
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

// src/_entries/faMelonSlice.js
var import_faMelonSlice = __toESM(require_faMelonSlice());
var export_faMelonSlice = import_faMelonSlice.definition;
export {
  export_faMelonSlice as faMelonSlice
};
