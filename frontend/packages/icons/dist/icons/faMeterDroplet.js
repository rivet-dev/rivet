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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faMeterDroplet.js
var require_faMeterDroplet = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faMeterDroplet.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "meter-droplet";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e1ea";
    var svgPathData = "M256 512c53.7 0 103.5-16.5 144.6-44.7C390 447.2 384 424.3 384 400c0-23.6 5.8-46.9 16.9-67.8l75.3-141.2c4.9-9.2 12.1-16.7 20.6-22.1C461.2 70.4 366.8 0 256 0C114.6 0 0 114.6 0 256S114.6 512 256 512zM192 160l0 32c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-32c0-17.7 14.3-32 32-32s32 14.3 32 32zm96 0l0 32c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-32c0-17.7 14.3-32 32-32s32 14.3 32 32zm96 0l0 32c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-32c0-17.7 14.3-32 32-32s32 14.3 32 32zm120.5 46.1L429.2 347.3C420.5 363.5 416 381.6 416 400c0 61.9 50.1 112 112 112s112-50.1 112-112c0-18.4-4.5-36.5-13.2-52.7L551.5 206.1c-4.6-8.7-13.7-14.1-23.5-14.1s-18.9 5.4-23.5 14.1z";
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
    exports.faMeterDroplet = exports.definition;
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

// src/_entries/faMeterDroplet.js
var import_faMeterDroplet = __toESM(require_faMeterDroplet());
var export_faMeterDroplet = import_faMeterDroplet.definition;
export {
  export_faMeterDroplet as faMeterDroplet
};
