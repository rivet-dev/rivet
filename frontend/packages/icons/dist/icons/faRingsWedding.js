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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faRingsWedding.js
var require_faRingsWedding = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faRingsWedding.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "rings-wedding";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f81b";
    var svgPathData = "M113.7 8.8C116.4 3.4 121.9 0 128 0l96 0c6.1 0 11.6 3.4 14.3 8.8l16 32c2.3 4.5 2.3 9.8 0 14.3l-24.6 49.2C300.6 127.1 352 193.5 352 272c0 52.3-22.8 99.2-59 131.5c-18.7-11.9-32.1-31.5-35.9-54.2C276.2 329.2 288 302 288 272c0-12-1.9-23.6-5.4-34.5c-34.9 19-58.6 56-58.6 98.5c0 61.9 50.1 112 112 112s112-50.1 112-112c0-46.1-27.8-85.7-67.6-102.9c-5-26.2-14.8-50.7-28.6-72.4C441.6 168.7 512 244.1 512 336c0 97.2-78.8 176-176 176c-55.3 0-104.7-25.5-137-65.5c-7.5 1-15.2 1.5-23 1.5C78.8 448 0 369.2 0 272c0-78.5 51.4-144.9 122.3-167.7L97.7 55.2c-2.3-4.5-2.3-9.8 0-14.3l16-32zM176 160c-61.9 0-112 50.1-112 112c0 58.7 45.1 106.8 102.5 111.6c-4.2-15.1-6.5-31.1-6.5-47.6c0-64.4 34.5-120.7 86.1-151.3C226.9 169.2 202.5 160 176 160z";
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
    exports.faRingsWedding = exports.definition;
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

// src/_entries/faRingsWedding.js
var import_faRingsWedding = __toESM(require_faRingsWedding());
var export_faRingsWedding = import_faRingsWedding.definition;
export {
  export_faRingsWedding as faRingsWedding
};
