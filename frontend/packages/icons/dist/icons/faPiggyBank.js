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

// src/node_modules/@fortawesome/free-solid-svg-icons/faPiggyBank.js
var require_faPiggyBank = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faPiggyBank.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "piggy-bank";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f4d3";
    var svgPathData = "M288-32a96 96 0 1 1 0 192 96 96 0 1 1 0-192zM48 304c0-70.1 47-131.4 117.1-164.9 25.3 41.3 70.9 68.9 122.9 68.9 55.7 0 104.1-31.7 128-78 15.8-11.3 35.1-18 56-18l19.5 0c10.4 0 18 9.8 15.5 19.9l-17.1 68.3c9.9 12.4 18.2 25.7 24.4 39.8l21.7 0c13.3 0 24 10.7 24 24l0 112c0 13.3-10.7 24-24 24l-40 0c-16.5 22-38.5 39.6-64 50.7l0 29.3c0 17.7-14.3 32-32 32l-33 0c-14.3 0-26.8-9.5-30.8-23.2l-7.1-24.8-82.3 0-7.1 24.8C235.8 502.5 223.3 512 209 512l-33 0c-17.7 0-32-14.3-32-32l0-29.3C87.5 426 48 369.6 48 304zm376 16a24 24 0 1 0 0-48 24 24 0 1 0 0 48z";
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
    exports.faPiggyBank = exports.definition;
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

// src/_entries/faPiggyBank.js
var import_faPiggyBank = __toESM(require_faPiggyBank());
var export_faPiggyBank = import_faPiggyBank.definition;
export {
  export_faPiggyBank as faPiggyBank
};
