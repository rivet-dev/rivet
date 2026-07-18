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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPipeCollar.js
var require_faPipeCollar = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPipeCollar.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "pipe-collar";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e437";
    var svgPathData = "M18.7 18.7c25-25 65.5-25 90.5 0L126 35.4C164.1 12.9 208.5 0 256 0s91.9 12.9 130 35.4l16.7-16.7c25-25 65.5-25 90.5 0s25 65.5 0 90.5L476.6 126c22.5 38.1 35.4 82.6 35.4 130s-12.9 91.9-35.4 130l16.7 16.7c25 25 25 65.5 0 90.5s-65.5 25-90.5 0L386 476.6C347.9 499.1 303.5 512 256 512s-91.9-12.9-130-35.4l-16.7 16.7c-25 25-65.5 25-90.5 0s-25-65.5 0-90.5L35.4 386C12.9 347.9 0 303.5 0 256s12.9-91.9 35.4-130L18.7 109.3c-25-25-25-65.5 0-90.5zM64 80a16 16 0 1 0 0-32 16 16 0 1 0 0 32zM256 416a160 160 0 1 0 0-320 160 160 0 1 0 0 320zM80 448a16 16 0 1 0 -32 0 16 16 0 1 0 32 0zM448 80a16 16 0 1 0 0-32 16 16 0 1 0 0 32zm16 368a16 16 0 1 0 -32 0 16 16 0 1 0 32 0z";
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
    exports.faPipeCollar = exports.definition;
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

// src/_entries/faPipeCollar.js
var import_faPipeCollar = __toESM(require_faPipeCollar());
var export_faPipeCollar = import_faPipeCollar.definition;
export {
  export_faPipeCollar as faPipeCollar
};
