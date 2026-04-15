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

// src/node_modules/@fortawesome/free-solid-svg-icons/faAnkh.js
var require_faAnkh = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faAnkh.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "ankh";
    var width = 320;
    var height = 512;
    var aliases = [9765];
    var unicode = "f644";
    var svgPathData = "M96 128c0-35.3 28.7-64 64-64s64 28.7 64 64c0 41.6-20.7 76.6-46.6 104.1-5.9 6.2-11.8 11.8-17.4 16.7-5.6-4.9-11.5-10.5-17.4-16.7-25.9-27.5-46.6-62.6-46.6-104.1zM160 0C89.3 0 32 57.3 32 128 32 180.4 53.5 223.5 78.8 256L32 256c-17.7 0-32 14.3-32 32s14.3 32 32 32l96 0 0 160c0 17.7 14.3 32 32 32s32-14.3 32-32l0-160 96 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-46.8 0C266.5 223.5 288 180.4 288 128 288 57.3 230.7 0 160 0z";
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
    exports.faAnkh = exports.definition;
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

// src/_entries/faAnkh.js
var import_faAnkh = __toESM(require_faAnkh());
var export_faAnkh = import_faAnkh.definition;
export {
  export_faAnkh as faAnkh
};
