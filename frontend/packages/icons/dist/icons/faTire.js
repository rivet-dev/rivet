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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTire.js
var require_faTire = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTire.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "tire";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f631";
    var svgPathData = "M256 512A256 256 0 1 0 256 0a256 256 0 1 0 0 512zm32-256a32 32 0 1 0 -64 0 32 32 0 1 0 64 0zm-16.1 62l0 65c44.3-5.5 81.5-33.6 99.8-72.4l-61.9-20.1c-8.6 13.5-22.1 23.5-38 27.6zm47.9-58l61.8 20.1c1.5-7.8 2.3-15.9 2.3-24.2c0-36.8-15.5-69.9-40.3-93.3l-38.2 52.6c9.1 11 14.5 25.2 14.5 40.6c0 1.4 0 2.7-.1 4zm-40.2-63.5l38.2-52.6C299.5 133.8 278.4 128 256 128s-43.5 5.8-61.8 15.9l28.1 38.8 10 13.8c7.3-2.9 15.3-4.5 23.6-4.5s16.3 1.6 23.6 4.5zM168.3 162.7C143.5 186.1 128 219.2 128 256c0 8.3 .8 16.3 2.3 24.2L192.1 260c-.1-1.3-.1-2.7-.1-4c0-15.4 5.5-29.6 14.5-40.6l-10.1-13.9-28.1-38.7zm33.7 127.7l-61.9 20.1c18.3 38.8 55.5 66.9 99.8 72.4l0-65c-15.8-4.1-29.3-14.1-37.9-27.5zM256 96a160 160 0 1 1 0 320 160 160 0 1 1 0-320z";
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
    exports.faTire = exports.definition;
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

// src/_entries/faTire.js
var import_faTire = __toESM(require_faTire());
var export_faTire = import_faTire.definition;
export {
  export_faTire as faTire
};
