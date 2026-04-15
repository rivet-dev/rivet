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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faRouter.js
var require_faRouter = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faRouter.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "router";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f8da";
    var svgPathData = "M352 48c58.8 0 112.5 21.8 153.4 57.9c9.9 8.8 25.1 7.8 33.9-2.1s7.8-25.1-2.1-33.9C487.8 26.4 422.9 0 352 0S216.2 26.4 166.9 69.9c-9.9 8.8-10.9 23.9-2.1 33.9s23.9 10.9 33.9 2.1C239.5 69.8 293.2 48 352 48zm24 200c0-13.3-10.7-24-24-24s-24 10.7-24 24l0 72L64 320c-35.3 0-64 28.7-64 64l0 64c0 35.3 28.7 64 64 64l448 0c35.3 0 64-28.7 64-64l0-64c0-35.3-28.7-64-64-64l-136 0 0-72zM96 384a32 32 0 1 1 0 64 32 32 0 1 1 0-64zm64 32a32 32 0 1 1 64 0 32 32 0 1 1 -64 0zM276.5 186.7C297.1 170 323.4 160 352 160s54.9 10 75.5 26.7c10.3 8.3 25.4 6.8 33.8-3.5s6.8-25.4-3.5-33.8C428.8 126 392 112 352 112s-76.8 14-105.7 37.4c-10.3 8.3-11.9 23.5-3.5 33.8s23.5 11.9 33.8 3.5z";
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
    exports.faRouter = exports.definition;
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

// src/_entries/faRouter.js
var import_faRouter = __toESM(require_faRouter());
var export_faRouter = import_faRouter.definition;
export {
  export_faRouter as faRouter
};
