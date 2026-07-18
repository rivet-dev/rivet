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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTimer.js
var require_faTimer = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTimer.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "timer";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e29e";
    var svgPathData = "M256 0c-17.7 0-32 14.3-32 32l0 64c0 17.7 14.3 32 32 32s32-14.3 32-32l0-29.3C378.8 81.9 448 160.9 448 256c0 106-86 192-192 192S64 362 64 256c0-53.7 22-102.3 57.6-137.1c12.6-12.4 12.8-32.6 .5-45.3S89.5 60.8 76.8 73.1C29.5 119.6 0 184.4 0 256C0 397.4 114.6 512 256 512s256-114.6 256-256S397.4 0 256 0zM193 159c-9.4-9.4-24.6-9.4-33.9 0s-9.4 24.6 0 33.9l80 80c9.4 9.4 24.6 9.4 33.9 0s9.4-24.6 0-33.9l-80-80z";
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
    exports.faTimer = exports.definition;
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

// src/_entries/faTimer.js
var import_faTimer = __toESM(require_faTimer());
var export_faTimer = import_faTimer.definition;
export {
  export_faTimer as faTimer
};
