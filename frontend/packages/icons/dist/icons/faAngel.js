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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faAngel.js
var require_faAngel = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faAngel.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "angel";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f779";
    var svgPathData = "M384 64c0 0 0 .1 0 .2s-.1 .3-.2 .6c-.3 .6-.9 1.8-2.3 3.5c-1.4 1.7-3.4 3.6-6.2 5.6c6.7 8.3 12.2 17.6 16.3 27.7C407 91 416 78 416 64c0-35.3-57.3-64-128-64S160 28.7 160 64c0 14 9 27 24.3 37.5c4.1-10 9.6-19.3 16.3-27.7c-2.7-2-4.8-4-6.2-5.6c-1.4-1.7-2-2.8-2.3-3.5c-.1-.3-.2-.5-.2-.6s0-.1 0-.2s0-.1 0-.2s.1-.3 .2-.6c.3-.6 .9-1.8 2.3-3.5c2.9-3.5 8.4-7.9 17.3-12.4C229.6 38.4 256.5 32 288 32s58.4 6.4 76.2 15.4c8.9 4.5 14.4 8.9 17.3 12.4c1.4 1.7 2 2.8 2.3 3.5c.1 .3 .2 .5 .2 .6s0 .1 0 .2zm-16 80a80 80 0 1 0 -160 0 80 80 0 1 0 160 0zM120 464l-70.3 0 37.9-75.8c5.6-11.1 8.4-23.4 8.4-35.8l0-.9c0-12.4-2.9-24.7-8.4-35.8L69.2 279c-3.4-6.7-5.2-14.4-5.2-22.1c0-27.1 22-48.9 48.2-48.9c12.8 0 25 5.1 34.1 14.1l66.4 66.4c-6.8 7.2-12.7 15.5-17.3 24.7L120 464zM253.8 261.8l-73.6-73.6c-18-18-42.5-28.2-68-28.2C59.1 160 16 203.8 16 256.9c0 15.1 3.5 30.1 10.2 43.6l18.4 36.7c2.2 4.4 3.4 9.3 3.4 14.3l0 .9c0 5-1.2 9.9-3.4 14.3L4.8 446.4C1.6 452.7 0 459.7 0 466.7c0 25 20.3 45.3 45.3 45.3L96 512l96 0 192 0 96 0 50.7 0c25 0 45.3-20.3 45.3-45.3c0-7-1.6-14-4.8-20.3l-39.8-79.7c-2.2-4.4-3.4-9.3-3.4-14.3l0-.9c0-5 1.2-9.9 3.4-14.3l18.4-36.7C556.5 287 560 272 560 256.9c0-53.1-43.1-96.9-96.2-96.9c-25.5 0-50 10.1-68 28.2l-73.6 73.6C311.4 258 299.8 256 288 256s-23.4 2-34.2 5.8zm126.8 51.4c-4.6-9.2-10.4-17.5-17.3-24.7l66.4-66.4c9-9 21.3-14.1 34.1-14.1c26.1 0 48.2 21.8 48.2 48.9c0 7.7-1.8 15.4-5.2 22.1l-18.4 36.7c-5.6 11.1-8.4 23.4-8.4 35.8l0 .9c0 12.4 2.9 24.7 8.4 35.8L526.3 464 456 464 380.6 313.2z";
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
    exports.faAngel = exports.definition;
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

// src/_entries/faAngel.js
var import_faAngel = __toESM(require_faAngel());
var export_faAngel = import_faAngel.definition;
export {
  export_faAngel as faAngel
};
