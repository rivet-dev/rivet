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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faOctopus.js
var require_faOctopus = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faOctopus.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "octopus";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e688";
    var svgPathData = "M207.8 359.6c-2.8 40.4-36.4 72.4-77.6 72.4c-36.7 0-68.3-25.6-76-61.5L47.5 339c-2.8-13-15.5-21.2-28.5-18.4S-2.2 336.1 .5 349l6.8 31.6c12.4 58 63.6 99.4 122.9 99.4c0 0 .1 0 .1 0l4.1 0c42.4 0 83.8-12.5 119-36l21.4-14.2c13.4-8.9 29.2-13.7 45.3-13.7s31.9 4.8 45.3 13.7L386.7 444c35.2 23.5 76.6 36 119 36l4.1 0c59.3 0 110.5-41.4 123-99.4l6.8-31.6c2.8-13-5.5-25.7-18.4-28.5s-25.7 5.5-28.5 18.4l-6.8 31.6c-7.7 35.8-39.4 61.5-76 61.5c-41.1 0-74.8-31.9-77.6-72.4c11.3 7.3 24.5 11.9 38.9 13c38.8 2.8 74.3-21.9 85.3-59.2l10.9-36.9c6.4-21.6 26.2-36.5 48.8-36.5c13.3 0 24-10.7 24-24s-10.7-24-24-24c-43.8 0-82.5 28.9-94.8 71l-10.8 36.9c-4.6 15.7-19.5 26-35.8 24.8c-20.5-1.5-35.3-20.3-31.7-40.6l19-109.3c1.5-8.4 2.2-17 2.2-25.6l0-5.2C464 64.5 399.5 0 320 0S176 64.5 176 144l0 5.2c0 8.6 .7 17.1 2.2 25.6l19 109.3c3.5 20.3-11.2 39.1-31.7 40.6c-16.3 1.2-31.2-9.2-35.8-24.8L118.8 263c-12.4-42.1-51-71-94.8-71c-13.3 0-24 10.7-24 24s10.7 24 24 24c22.6 0 42.4 14.9 48.8 36.5l10.9 36.9c11 37.3 46.5 62 85.3 59.2c14.4-1 27.7-5.7 38.9-13zM240 272a16 16 0 1 1 32 0 16 16 0 1 1 -32 0zm144-16a16 16 0 1 1 0 32 16 16 0 1 1 0-32z";
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
    exports.faOctopus = exports.definition;
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

// src/_entries/faOctopus.js
var import_faOctopus = __toESM(require_faOctopus());
var export_faOctopus = import_faOctopus.definition;
export {
  export_faOctopus as faOctopus
};
