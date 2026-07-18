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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPlayPause.js
var require_faPlayPause = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPlayPause.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "play-pause";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e22f";
    var svgPathData = "M116.5 71.4c-9.5-7.9-22.8-9.7-34.1-4.4S64 83.6 64 96l0 320c0 12.4 7.2 23.7 18.4 29s24.5 3.6 34.1-4.4l192-160c7.3-6.1 11.5-15.1 11.5-24.6s-4.2-18.5-11.5-24.6l-192-160zM448 96c0-17.7-14.3-32-32-32s-32 14.3-32 32l0 320c0 17.7 14.3 32 32 32s32-14.3 32-32l0-320zm128 0c0-17.7-14.3-32-32-32s-32 14.3-32 32l0 320c0 17.7 14.3 32 32 32s32-14.3 32-32l0-320z";
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
    exports.faPlayPause = exports.definition;
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

// src/_entries/faPlayPause.js
var import_faPlayPause = __toESM(require_faPlayPause());
var export_faPlayPause = import_faPlayPause.definition;
export {
  export_faPlayPause as faPlayPause
};
