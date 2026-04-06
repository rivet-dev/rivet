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

// src/node_modules/@fortawesome/free-solid-svg-icons/faBong.js
var require_faBong = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faBong.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bong";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f55c";
    var svgPathData = "M448.5 384c0 41.5-13.1 79.9-35.5 111.3-7.9 11.1-21.2 16.7-34.8 16.7l-243.3 0c-13.7 0-26.9-5.6-34.8-16.7-22.4-31.4-35.5-69.8-35.5-111.3 0-71.1 38.6-133.1 96-166.3l0-153.7c-17.7 0-32-14.3-32-32s14.3-32 32-32l192 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l0 153.7c14.2 8.2 27.2 18.2 38.8 29.6l39.3-39.3-7-7c-9.4-9.4-9.4-24.6 0-33.9s24.6-9.4 33.9 0c16 16 32 32 48 48 9.4 9.4 9.4 24.6 0 33.9s-24.6 9.4-33.9 0l-7-7-43.3 43.3c17.3 28.9 27.3 62.6 27.3 98.8zM288.5 64l-64 0 0 190.6-32 18.5c-19.5 11.3-35.7 27.5-46.9 46.9l221.8 0c-11.2-19.4-27.4-35.6-46.9-46.9l-32-18.5 0-190.6z";
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
    exports.faBong = exports.definition;
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

// src/_entries/faBong.js
var import_faBong = __toESM(require_faBong());
var export_faBong = import_faBong.definition;
export {
  export_faBong as faBong
};
