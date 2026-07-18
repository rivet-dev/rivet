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

// src/node_modules/@fortawesome/free-solid-svg-icons/faBacterium.js
var require_faBacterium = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faBacterium.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bacterium";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e05a";
    var svgPathData = "M408 16c0-13.3-10.7-24-24-24S360 2.7 360 16l0 23.4c-13.9 5.3-26.9 13.5-38.2 24.5L305 47c-9.4-9.4-24.6-9.4-33.9 0s-9.4 24.6 0 33.9l16.8 16.8-30.1 30.1-16.8-16.8c-9.4-9.4-24.6-9.4-33.9 0s-9.4 24.6 0 33.9l16.8 16.8c-10 10-20 20-30.1 30.1L177 175c-9.4-9.4-24.6-9.4-33.9 0s-9.4 24.6 0 33.9l16.8 16.8-30.1 30.1-16.8-16.8c-9.4-9.4-24.6-9.4-33.9 0s-9.4 24.6 0 33.9l16.8 16.8c-11 11.3-19.2 24.3-24.5 38.2L48 328c-13.3 0-24 10.7-24 24s10.7 24 24 24l16.3 0c1.3 18.3 7.1 36.4 17.4 52.4L71 439c-9.4 9.4-9.4 24.6 0 33.9s24.6 9.4 33.9 0l10.6-10.6c16 10.3 34.1 16.1 52.4 17.4l0 16.3c0 13.3 10.7 24 24 24s24-10.7 24-24l0-23.4c13.9-5.3 26.9-13.5 38.2-24.5L271 465c9.4 9.4 24.6 9.4 33.9 0s9.4-24.6 0-33.9l-16.8-16.8 30.1-30.1 16.8 16.8c9.4 9.4 24.6 9.4 33.9 0s9.4-24.6 0-33.9l-16.8-16.8c10-10 20-20 30.1-30.1L399 337c9.4 9.4 24.6 9.4 33.9 0s9.4-24.6 0-33.9l-16.8-16.8 30.1-30.1 16.8 16.8c9.4 9.4 24.6 9.4 33.9 0s9.4-24.6 0-33.9l-16.8-16.8c11-11.3 19.2-24.3 24.5-38.2l23.4 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-16.3 0c-1.3-18.3-7.1-36.4-17.4-52.4L505 73c9.4-9.4 9.4-24.6 0-33.9s-24.6-9.4-33.9 0L460.4 49.7c-16-10.3-34.1-16.1-52.4-17.4L408 16zM160 352a32 32 0 1 1 64 0 32 32 0 1 1 -64 0zM272 240a32 32 0 1 1 0 64 32 32 0 1 1 0-64z";
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
    exports.faBacterium = exports.definition;
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

// src/_entries/faBacterium.js
var import_faBacterium = __toESM(require_faBacterium());
var export_faBacterium = import_faBacterium.definition;
export {
  export_faBacterium as faBacterium
};
