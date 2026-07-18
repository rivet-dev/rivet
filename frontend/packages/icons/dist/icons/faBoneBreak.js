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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBoneBreak.js
var require_faBoneBreak = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBoneBreak.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bone-break";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f5d8";
    var svgPathData = "M344 24c0-13.3-10.7-24-24-24s-24 10.7-24 24l0 80c0 13.3 10.7 24 24 24s24-10.7 24-24l0-80zM201 71c-9.4-9.4-24.6-9.4-33.9 0s-9.4 24.6 0 33.9l48 48c9.4 9.4 24.6 9.4 33.9 0s9.4-24.6 0-33.9L201 71zM473 105c9.4-9.4 9.4-24.6 0-33.9s-24.6-9.4-33.9 0l-48 48c-9.4 9.4-9.4 24.6 0 33.9s24.6 9.4 33.9 0l48-48zM143.9 223.8C129.3 204.5 106.1 192 80 192c-44.2 0-80 35.8-80 80c0 35.8 23.5 66.1 55.9 76.3c4.6 1.5 8.3 5.1 9.3 9.8s-1.1 9.5-4.8 12.6C43.1 385.5 32 407.5 32 432c0 44.2 35.8 80 80 80s80-35.8 80-80c0-11.9 7-23.8 18.6-26.1L320 384l109.4 21.9C441 408.2 448 420.1 448 432c0 44.2 35.8 80 80 80s80-35.8 80-80c0-24.5-11.1-46.5-28.5-61.2c-3.7-3.1-5.7-7.9-4.8-12.6s4.7-8.4 9.3-9.8C616.5 338.1 640 307.8 640 272c0-44.2-35.8-80-80-80c-26.1 0-49.3 12.5-63.9 31.8c-10.1 13.3-25.7 23.8-42.1 20.5l-70.6-14.1c-17.3-3.5-34.2 7.8-37.7 25.1L320 384 294.3 255.4c-3.5-17.3-20.3-28.6-37.7-25.1L186 244.4c-16.4 3.3-32.1-7.2-42.1-20.5z";
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
    exports.faBoneBreak = exports.definition;
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

// src/_entries/faBoneBreak.js
var import_faBoneBreak = __toESM(require_faBoneBreak());
var export_faBoneBreak = import_faBoneBreak.definition;
export {
  export_faBoneBreak as faBoneBreak
};
