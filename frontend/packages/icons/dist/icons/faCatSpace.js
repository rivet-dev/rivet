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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCatSpace.js
var require_faCatSpace = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCatSpace.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "cat-space";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e001";
    var svgPathData = "M576 160c0 70.7-57.3 128-128 128s-128-57.3-128-128c0-32.5 12.1-62.1 32-84.7l0 84.7c0 53 43 96 96 96s96-43 96-96l0-84.7c19.9 22.6 32 52.2 32 84.7zM411.2 89.6L381.8 50.4C401.1 38.7 423.8 32 448 32s46.9 6.7 66.2 18.4L484.8 89.6c-3 4-7.8 6.4-12.8 6.4l-48 0c-5 0-9.8-2.4-12.8-6.4zM480 316.8C553 302 608 237.4 608 160C608 71.6 536.4 0 448 0S288 71.6 288 160c0 11.7 1.3 23.1 3.6 34.1c-54.9 8.1-102.2 39.5-131.6 83.8l0-85.3c0-48.4-36.1-89.3-84.1-95.3l-7.9-1C50.4 94.1 34.4 106.5 32.2 124s10.2 33.5 27.8 35.7l7.9 1c16 2 28 15.6 28 31.8L96 416c0 53 43 96 96 96l144 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-56 0L416 339.2 416 480c0 17.7 14.3 32 32 32s32-14.3 32-32l0-163.2zM400 160a16 16 0 1 1 32 0 16 16 0 1 1 -32 0zm80-16a16 16 0 1 1 0 32 16 16 0 1 1 0-32zM231.5 12.3c-2.6-7-12.4-7-15 0L205.8 41.1c-.8 2.2-2.5 3.9-4.7 4.7L172.3 56.5c-7 2.6-7 12.4 0 15l28.8 10.7c2.2 .8 3.9 2.5 4.7 4.7l10.7 28.8c2.6 7 12.4 7 15 0l10.7-28.8c.8-2.2 2.5-3.9 4.7-4.7l28.8-10.7c7-2.6 7-12.4 0-15L246.9 45.8c-2.2-.8-3.9-2.5-4.7-4.7L231.5 12.3z";
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
    exports.faCatSpace = exports.definition;
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

// src/_entries/faCatSpace.js
var import_faCatSpace = __toESM(require_faCatSpace());
var export_faCatSpace = import_faCatSpace.definition;
export {
  export_faCatSpace as faCatSpace
};
