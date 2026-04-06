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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSquareS.js
var require_faSquareS = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSquareS.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "square-s";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e27d";
    var svgPathData = "M64 32C28.7 32 0 60.7 0 96L0 416c0 35.3 28.7 64 64 64l320 0c35.3 0 64-28.7 64-64l0-320c0-35.3-28.7-64-64-64L64 32zM175.6 196.2c-1.1 6.4-.2 9.7 .6 11.6c1 2 2.9 4.6 7.4 7.7c10.1 6.8 25.7 11.5 46.8 17.4l2 .6s0 0 0 0c18.4 5.2 41.4 11.7 58.6 23.2c9.5 6.4 18.5 15.1 24.1 27.2c5.7 12.3 7 25.9 4.4 40.3c-4.7 26.3-23.1 43.3-45 52c-21.3 8.4-47 9.6-72.6 5.7l-.1 0s0 0 0 0c-16.3-2.6-43.7-10.7-57.3-15.1c-12.6-4-19.6-17.6-15.5-30.2s17.6-19.6 30.2-15.5c13.9 4.5 37.8 11.4 50 13.4c20.2 3 36.8 1.4 47.7-2.9c10.3-4.1 14.2-9.6 15.4-15.8c1.1-6.4 .2-9.7-.6-11.6c-1-2-2.9-4.6-7.4-7.7c-10.1-6.8-25.7-11.5-46.8-17.4l-2-.6c-18.4-5.2-41.4-11.7-58.6-23.2c-9.5-6.4-18.5-15.1-24.1-27.2c-5.7-12.3-7-25.9-4.4-40.3c4.7-26.3 23.1-43.3 45-52c21.3-8.4 47-9.6 72.6-5.7c8.1 1.2 24.4 4.8 32 6.7c12.8 3.3 20.6 16.4 17.3 29.2s-16.4 20.6-29.2 17.3c-6.7-1.7-21.3-4.9-27.3-5.7c-20.3-3.1-36.8-1.4-47.8 2.9c-10.3 4.1-14.2 9.6-15.4 15.8z";
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
    exports.faSquareS = exports.definition;
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

// src/_entries/faSquareS.js
var import_faSquareS = __toESM(require_faSquareS());
var export_faSquareS = import_faSquareS.definition;
export {
  export_faSquareS as faSquareS
};
