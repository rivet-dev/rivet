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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faGolfFlagHole.js
var require_faGolfFlagHole = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faGolfFlagHole.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "golf-flag-hole";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e3ac";
    var svgPathData = "M162.7 2.5c-7.4-3.7-16.3-3.3-23.4 1.1S128 15.7 128 24l0 208c0 1.3 .1 2.7 .3 4c-.2 1.3-.3 2.6-.3 4l0 92.8C51.5 349.4 0 380.5 0 416c0 53 114.6 96 256 96s256-43 256-96s-114.6-96-256-96c-27.9 0-54.8 1.7-80 4.8l0-77.9 194.7-97.4c8.1-4.1 13.3-12.4 13.3-21.5s-5.1-17.4-13.3-21.5l-208-104zM288 416c0 17.7-28.7 32-64 32s-64-14.3-64-32s28.7-32 64-32s64 14.3 64 32z";
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
    exports.faGolfFlagHole = exports.definition;
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

// src/_entries/faGolfFlagHole.js
var import_faGolfFlagHole = __toESM(require_faGolfFlagHole());
var export_faGolfFlagHole = import_faGolfFlagHole.definition;
export {
  export_faGolfFlagHole as faGolfFlagHole
};
