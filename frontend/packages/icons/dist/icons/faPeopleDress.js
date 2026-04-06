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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPeopleDress.js
var require_faPeopleDress = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPeopleDress.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "people-dress";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e217";
    var svgPathData = "M208 48a48 48 0 1 0 -96 0 48 48 0 1 0 96 0zM70.2 384L88 384l0 96c0 17.7 14.3 32 32 32s32-14.3 32-32l0-96 16 0 0 96c0 17.7 14.3 32 32 32s32-14.3 32-32l0-96 17.8 0c10.9 0 18.6-10.7 15.2-21.1L226.7 248.1l33.9 56.3c9.1 15.1 28.8 20 43.9 10.9c9.8-5.9 15.3-16.2 15.5-26.9c.2 10.7 5.7 21 15.5 26.9c15.1 9.1 34.8 4.2 43.9-10.9l33.9-56.3L375 362.9c-3.5 10.4 4.3 21.1 15.2 21.1l17.8 0 0 96c0 17.7 14.3 32 32 32s32-14.3 32-32l0-96 16 0 0 96c0 17.7 14.3 32 32 32s32-14.3 32-32l0-96 17.8 0c10.9 0 18.6-10.7 15.2-21.1L546.7 248.1l33.9 56.3c9.1 15.1 28.8 20 43.9 10.9s20-28.8 10.9-43.9l-53.6-89.2c-20.2-33.7-56.7-54.3-96-54.3l-11.6 0c-39.3 0-75.7 20.6-96 54.3l-53.6 89.2c-3 5-4.5 10.5-4.6 15.9c-.1-5.4-1.6-10.9-4.6-15.9l-53.6-89.2c-20.3-33.7-56.7-54.3-96-54.3l-11.6 0c-39.3 0-75.7 20.6-96 54.3L4.6 271.5c-9.1 15.1-4.2 34.8 10.9 43.9s34.8 4.2 43.9-10.9l33.9-56.3L55 362.9C51.6 373.3 59.3 384 70.2 384zM528 48a48 48 0 1 0 -96 0 48 48 0 1 0 96 0z";
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
    exports.faPeopleDress = exports.definition;
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

// src/_entries/faPeopleDress.js
var import_faPeopleDress = __toESM(require_faPeopleDress());
var export_faPeopleDress = import_faPeopleDress.definition;
export {
  export_faPeopleDress as faPeopleDress
};
