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

// src/node_modules/@fortawesome/free-solid-svg-icons/faGlassWaterDroplet.js
var require_faGlassWaterDroplet = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faGlassWaterDroplet.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "glass-water-droplet";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "e4f5";
    var svgPathData = "M32 0C23.1 0 14.6 3.7 8.6 10.2S-.6 25.4 .1 34.3L28.9 437.7c3 41.9 37.8 74.3 79.8 74.3l166.6 0c42 0 76.8-32.4 79.8-74.3L383.9 34.3c.6-8.9-2.4-17.6-8.5-24.1S360.9 0 352 0L32 0zM83 297.5L66.4 64 317.6 64 301 297.5 288 304c-20.1 10.1-43.9 10.1-64 0s-43.9-10.1-64 0-43.9 10.1-64 0l-13-6.5zM256 196c0-24-33.7-70.1-52.2-93.5-6.1-7.7-17.5-7.7-23.6 0-18.5 23.4-52.2 69.5-52.2 93.5 0 33.1 28.7 60 64 60s64-26.9 64-60z";
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
    exports.faGlassWaterDroplet = exports.definition;
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

// src/_entries/faGlassWaterDroplet.js
var import_faGlassWaterDroplet = __toESM(require_faGlassWaterDroplet());
var export_faGlassWaterDroplet = import_faGlassWaterDroplet.definition;
export {
  export_faGlassWaterDroplet as faGlassWaterDroplet
};
